// https://wiki.openstreetmap.org/wiki/PBF_Format
use flate2::read::ZlibDecoder;
use std::io::Read;

pub enum Element<'a> {
  Node {
    id: i64,
    lat: f64,
    lng: f64,
    tags: Vec<(&'a str, &'a str)>,
  },
  Way {
    id: i64,
    refs: Vec<i64>,
    tags: Vec<(&'a str, &'a str)>,
  },
  /// Way members only
  Relation {
    id: i64,
    ways: Vec<i64>,
    tags: Vec<(&'a str, &'a str)>,
  },
}

#[derive(Clone, Copy)]
pub struct Want {
  pub nodes: bool,
  pub ways: bool,
  pub relations: bool,
}

pub fn read(bytes: &[u8], want: Want, mut f: impl FnMut(Element<'_>)) -> Result<(), String> {
  let mut buf = Vec::new();
  let mut pos = 0;
  while pos < bytes.len() {
    let len = bytes.get(pos..pos + 4).ok_or("truncated blob header length")?;
    let len = u32::from_be_bytes([len[0], len[1], len[2], len[3]]) as usize;
    pos += 4;
    let header = bytes.get(pos..pos + len).ok_or("truncated blob header")?;
    pos += len;

    let (mut kind, mut size) = (&[][..], 0);
    for (field, value) in fields(header) {
      match (field, value) {
        (1, Value::Bytes(b)) => kind = b,
        (3, Value::Int(n)) => size = n as usize,
        _ => {}
      }
    }
    let blob = bytes.get(pos..pos + size).ok_or("truncated blob")?;
    pos += size;

    if kind == &b"OSMData"[..] {
      inflate(blob, &mut buf)?;
      block(&buf, want, &mut f);
    }
  }
  Ok(())
}

fn inflate(blob: &[u8], out: &mut Vec<u8>) -> Result<(), String> {
  out.clear();
  for (field, value) in fields(blob) {
    match (field, value) {
      (1, Value::Bytes(raw)) => out.extend_from_slice(raw),
      (2, Value::Int(n)) => out.reserve(n as usize),
      (3, Value::Bytes(zlib)) => {
        ZlibDecoder::new(zlib).read_to_end(out).map_err(|e| format!("zlib: {e}"))?;
      }
      (4..=7, _) => return Err("unsupported blob compression".into()),
      _ => {}
    }
  }
  Ok(())
}

fn block(data: &[u8], want: Want, f: &mut impl FnMut(Element<'_>)) {
  let mut strings = Vec::new();
  let mut groups = Vec::new();
  let (mut granularity, mut lat_offset, mut lng_offset) = (100_i64, 0_i64, 0_i64);
  for (field, value) in fields(data) {
    match (field, value) {
      (1, Value::Bytes(table)) => {
        strings = fields(table)
          .filter_map(|(n, v)| match (n, v) {
            (1, Value::Bytes(s)) => Some(std::str::from_utf8(s).unwrap_or("")),
            _ => None,
          })
          .collect();
      }
      (2, Value::Bytes(group)) => groups.push(group),
      (17, Value::Int(n)) => granularity = n as i64,
      (19, Value::Int(n)) => lat_offset = n as i64,
      (20, Value::Int(n)) => lng_offset = n as i64,
      _ => {}
    }
  }

  let s = |i: u64| strings.get(i as usize).copied().unwrap_or("");
  let lat = |raw: i64| 1e-9 * (lat_offset + granularity * raw) as f64;
  let lng = |raw: i64| 1e-9 * (lng_offset + granularity * raw) as f64;

  for group in groups {
    for (field, value) in fields(group) {
      let Value::Bytes(b) = value else { continue };
      match field {
        1 if want.nodes => {
          let (mut id, mut la, mut lo, mut keys, mut vals) = (0, 0, 0, &[][..], &[][..]);
          for (n, v) in fields(b) {
            match (n, v) {
              (1, Value::Int(x)) => id = zigzag(x),
              (2, Value::Bytes(p)) => keys = p,
              (3, Value::Bytes(p)) => vals = p,
              (8, Value::Int(x)) => la = zigzag(x),
              (9, Value::Int(x)) => lo = zigzag(x),
              _ => {}
            }
          }
          let tags = packed(keys).zip(packed(vals)).map(|(k, v)| (s(k), s(v))).collect();
          f(Element::Node { id, lat: lat(la), lng: lng(lo), tags });
        }
        2 if want.nodes => {
          let (mut ids, mut lats, mut lngs, mut kv) = (&[][..], &[][..], &[][..], &[][..]);
          for (n, v) in fields(b) {
            match (n, v) {
              (1, Value::Bytes(p)) => ids = p,
              (8, Value::Bytes(p)) => lats = p,
              (9, Value::Bytes(p)) => lngs = p,
              (10, Value::Bytes(p)) => kv = p,
              _ => {}
            }
          }
          let mut kv = packed(kv);
          let (mut id, mut la, mut lo) = (0, 0, 0);
          for ((di, dla), dlo) in packed(ids).zip(packed(lats)).zip(packed(lngs)) {
            id += zigzag(di);
            la += zigzag(dla);
            lo += zigzag(dlo);
            let mut tags = Vec::new();
            while let Some(k) = kv.next().filter(|&k| k != 0) {
              tags.push((s(k), s(kv.next().unwrap_or(0))));
            }
            f(Element::Node { id, lat: lat(la), lng: lng(lo), tags });
          }
        }
        3 if want.ways => {
          let (mut id, mut keys, mut vals, mut refs) = (0, &[][..], &[][..], &[][..]);
          for (n, v) in fields(b) {
            match (n, v) {
              (1, Value::Int(x)) => id = x as i64,
              (2, Value::Bytes(p)) => keys = p,
              (3, Value::Bytes(p)) => vals = p,
              (8, Value::Bytes(p)) => refs = p,
              _ => {}
            }
          }
          let tags = packed(keys).zip(packed(vals)).map(|(k, v)| (s(k), s(v))).collect();
          let mut node = 0;
          let refs = packed(refs)
            .map(|d| {
              node += zigzag(d);
              node
            })
            .collect();
          f(Element::Way { id, refs, tags });
        }
        4 if want.relations => {
          let (mut id, mut keys, mut vals, mut ids, mut types) = (0, &[][..], &[][..], &[][..], &[][..]);
          for (n, v) in fields(b) {
            match (n, v) {
              (1, Value::Int(x)) => id = x as i64,
              (2, Value::Bytes(p)) => keys = p,
              (3, Value::Bytes(p)) => vals = p,
              (9, Value::Bytes(p)) => ids = p,
              (10, Value::Bytes(p)) => types = p,
              _ => {}
            }
          }
          let tags = packed(keys).zip(packed(vals)).map(|(k, v)| (s(k), s(v))).collect();
          let mut member = 0;
          let ways = packed(ids)
            .zip(packed(types))
            .filter_map(|(d, kind)| {
              member += zigzag(d);
              (kind == 1).then_some(member)
            })
            .collect();
          f(Element::Relation { id, ways, tags });
        }
        _ => {}
      }
    }
  }
}

// protobuf
enum Value<'a> {
  Int(u64),
  Bytes(&'a [u8]),
}

fn varint(buf: &[u8], pos: &mut usize) -> Option<u64> {
  let mut out = 0_u64;
  for shift in (0..64).step_by(7) {
    let byte = *buf.get(*pos)?;
    *pos += 1;
    out |= u64::from(byte & 0x7f) << shift;
    if byte & 0x80 == 0 {
      return Some(out);
    }
  }
  None
}

fn fields(buf: &[u8]) -> impl Iterator<Item = (u32, Value<'_>)> {
  let mut pos = 0;
  std::iter::from_fn(move || {
    let key = varint(buf, &mut pos)?;
    let value = match key & 7 {
      0 => Value::Int(varint(buf, &mut pos)?),
      1 => {
        pos += 8;
        Value::Int(0)
      }
      2 => {
        let len = varint(buf, &mut pos)? as usize;
        let bytes = buf.get(pos..pos + len)?;
        pos += len;
        Value::Bytes(bytes)
      }
      5 => {
        pos += 4;
        Value::Int(0)
      }
      _ => return None,
    };
    Some(((key >> 3) as u32, value))
  })
}

fn packed(buf: &[u8]) -> impl Iterator<Item = u64> + '_ {
  let mut pos = 0;
  std::iter::from_fn(move || varint(buf, &mut pos))
}

fn zigzag(n: u64) -> i64 {
  ((n >> 1) as i64) ^ -((n & 1) as i64)
}
