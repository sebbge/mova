use super::pbf::{self, Element, Want};
use crate::entities::place::{PlaceKind, search_key};

use std::collections::{HashMap, HashSet};

pub struct Row {
  id: String,
  kind: PlaceKind,
  name: String,
  category: &'static str,
  context: Option<String>,
  key: String,
  crossing: Option<String>,
  rank: i16,
  lat: f64,
  lng: f64,
}

/// 1e-7 degrees, as OSM publishes
type Coord = [i32; 2];
const UNSEEN: Coord = [i32::MIN, i32::MIN];

const STREET_RANK: i16 = 4;
const CORNER_RANK: i16 = 5;
/// Same-name segments closer than this are one street
const SAME_STREET_DEG: i32 = 20_000;
/// Crossings of the same streets closer than this are one corner
const SAME_CORNER_M: f64 = 150.0;
/// Zone context reach
const CONTEXT_REACH_M: f64 = 3000.0;
const CONTEXT_MAX_RANK: i16 = 1;

struct Named {
  id: i64,
  name: String,
  refs: Vec<i64>,
}

struct Street {
  way: Named,
  class: u8,
}

struct Feature {
  named: Named,
  kind: PlaceKind,
  category: &'static str,
  rank: i16,
}

pub fn extract(bytes: &[u8]) -> Result<Vec<Row>, String> {
  let mut streets = Vec::new();
  let mut features = Vec::new();
  let mut coords: HashMap<i64, Coord> = HashMap::new();

  let mut relations = Vec::new();
  let mut outlines: HashMap<i64, Vec<i64>> = HashMap::new();
  pbf::read(bytes, Want { nodes: false, ways: false, relations: true }, |e| {
    let Element::Relation { id, ways, tags } = e else { return };
    let (Some(name), Some((kind, category, rank))) = (name(&tags), classify(&tags)) else { return };
    outlines.extend(ways.iter().map(|&w| (w, Vec::new())));
    relations.push(Feature { named: Named { id, name: name.to_string(), refs: ways }, kind, category, rank });
  })?;

  pbf::read(bytes, Want { nodes: false, ways: true, relations: false }, |e| {
    let Element::Way { id, refs, tags } = e else { return };
    if let Some(outline) = outlines.get_mut(&id) {
      coords.extend(refs.iter().map(|&n| (n, UNSEEN)));
      outline.clone_from(&refs);
    }
    let Some(name) = name(&tags) else { return };
    let named = Named { id, name: name.to_string(), refs };
    if let Some(class) = tags.iter().find(|(k, _)| *k == "highway").and_then(|(_, v)| street_class(v)) {
      coords.extend(named.refs.iter().map(|&n| (n, UNSEEN)));
      streets.push(Street { way: named, class });
    } else if let Some((kind, category, rank)) = classify(&tags) {
      coords.extend(named.refs.iter().map(|&n| (n, UNSEEN)));
      features.push(Feature { named, kind, category, rank });
    }
  })?;

  let mut rows = Vec::new();
  pbf::read(bytes, Want { nodes: true, ways: false, relations: false }, |e| {
    let Element::Node { id, lat, lng, tags } = e else { return };
    if let Some(c) = coords.get_mut(&id) {
      *c = [(lat * 1e7).round() as i32, (lng * 1e7).round() as i32];
    }
    let (Some(name), Some((kind, category, rank))) = (name(&tags), classify(&tags)) else { return };
    rows.push(row(format!("n{id}"), kind, name.to_string(), category, rank, None, [lat, lng]));
  })?;

  for f in &features {
    let Some(at) = centroid(&f.named.refs, &coords) else { continue };
    rows.push(row(format!("w{}", f.named.id), f.kind, f.named.name.clone(), f.category, f.rank, None, at));
  }
  for f in &relations {
    let refs: Vec<i64> = f.named.refs.iter().filter_map(|w| outlines.get(w)).flatten().copied().collect();
    let Some(at) = centroid(&refs, &coords) else { continue };
    rows.push(row(format!("r{}", f.named.id), f.kind, f.named.name.clone(), f.category, f.rank, None, at));
  }
  rows.extend(streets_and_corners(&streets, &coords));

  let zones = Zones::new(&rows);
  for r in rows.iter_mut().filter(|r| r.kind != PlaceKind::Zone) {
    r.context = zones.nearest([r.lat, r.lng]).map(str::to_string);
  }

  let mut seen = HashSet::new();
  rows.retain(|r| seen.insert(r.id.clone()));
  Ok(rows)
}

fn row(
  id: String,
  kind: PlaceKind,
  name: String,
  category: &'static str,
  rank: i16,
  crossing: Option<&str>,
  [lat, lng]: [f64; 2],
) -> Row {
  let key = search_key(&name);
  Row { id, kind, name, category, context: None, key, crossing: crossing.map(search_key), rank, lat, lng }
}

fn name<'a>(tags: &[(&str, &'a str)]) -> Option<&'a str> {
  tags.iter().find(|(k, _)| *k == "name").map(|(_, v)| v.trim()).filter(|v| !v.is_empty())
}

fn street_class(highway: &str) -> Option<u8> {
  match highway {
    "motorway" | "trunk" | "primary" => Some(0),
    "secondary" => Some(1),
    "tertiary" => Some(2),
    "unclassified" | "residential" | "living_street" | "pedestrian" | "road" => Some(3),
    _ => None,
  }
}

fn classify(tags: &[(&str, &str)]) -> Option<(PlaceKind, &'static str, i16)> {
  use PlaceKind::{Poi, Zone};
  tags.iter().find_map(|&tag| {
    Some(match tag {
      ("place", "city" | "town") => (Zone, "Ciudad", 0),
      ("place", "village") => (Zone, "Pueblo", 1),
      ("place", "suburb" | "quarter") => (Zone, "Barrio", 1),
      ("place", "neighbourhood") => (Zone, "Barrio", 2),
      ("place", "hamlet" | "locality") => (Zone, "Localidad", 3),
      ("amenity", "hospital") => (Poi, "Hospital", 2),
      ("amenity", "clinic") => (Poi, "Policlínica", 2),
      ("amenity", "university") => (Poi, "Universidad", 2),
      ("amenity", "college") => (Poi, "Instituto", 2),
      ("amenity", "school") => (Poi, "Centro educativo", 2),
      ("amenity", "library") => (Poi, "Biblioteca", 2),
      ("amenity", "theatre") => (Poi, "Teatro", 2),
      ("amenity", "cinema") => (Poi, "Cine", 2),
      ("amenity", "arts_centre" | "community_centre") => (Poi, "Centro cultural", 2),
      ("amenity", "townhall") => (Poi, "Edificio público", 2),
      ("amenity", "courthouse") => (Poi, "Juzgado", 2),
      ("amenity", "police") => (Poi, "Policía", 2),
      ("amenity", "fire_station") => (Poi, "Bomberos", 2),
      ("amenity", "post_office") => (Poi, "Correo", 2),
      ("amenity", "marketplace") => (Poi, "Mercado", 2),
      ("amenity", "place_of_worship") => (Poi, "Templo", 2),
      ("amenity", "bus_station") => (Poi, "Terminal", 2),
      ("amenity", "ferry_terminal") => (Poi, "Puerto", 2),
      ("tourism", "museum") => (Poi, "Museo", 2),
      ("tourism", "attraction") => (Poi, "Atractivo", 2),
      ("tourism", "zoo") => (Poi, "Zoológico", 2),
      ("tourism", "viewpoint") => (Poi, "Mirador", 2),
      ("tourism", "hotel") => (Poi, "Hotel", 2),
      ("tourism", "hostel") => (Poi, "Hostel", 2),
      ("leisure", "park") => (Poi, "Parque", 2),
      ("leisure", "stadium") => (Poi, "Estadio", 2),
      ("leisure", "sports_centre") => (Poi, "Centro deportivo", 2),
      ("natural", "beach") => (Poi, "Playa", 2),
      ("shop", "mall") => (Poi, "Shopping", 2),
      ("shop", "supermarket") => (Poi, "Supermercado", 2),
      ("aeroway", "aerodrome") => (Poi, "Aeropuerto", 2),
      ("railway", "station") => (Poi, "Estación", 2),
      _ => return None,
    })
  })
}

fn centroid(refs: &[i64], coords: &HashMap<i64, Coord>) -> Option<[f64; 2]> {
  let seen: Vec<Coord> = refs.iter().filter_map(|n| coords.get(n)).filter(|c| **c != UNSEEN).copied().collect();
  if seen.is_empty() {
    return None;
  }
  let n = seen.len() as f64;
  let (lat, lng) = seen.iter().fold((0.0, 0.0), |(a, b), c| (a + f64::from(c[0]), b + f64::from(c[1])));
  Some([lat / n * 1e-7, lng / n * 1e-7])
}

/// Streets from touching same-name segments, corners from shared nodes
fn streets_and_corners(streets: &[Street], coords: &HashMap<i64, Coord>) -> Vec<Row> {
  let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
  for (i, s) in streets.iter().enumerate() {
    by_name.entry(&s.way.name).or_default().push(i);
  }

  let mut uf = UnionFind::new(streets.len());
  for ways in by_name.values() {
    let mut first: HashMap<i64, usize> = HashMap::new();
    for &i in ways {
      for &n in &streets[i].way.refs {
        let j = *first.entry(n).or_insert(i);
        uf.union(i, j);
      }
    }

    let boxes: Vec<(usize, [i32; 4])> =
      ways.iter().filter_map(|&i| bbox(&streets[i].way.refs, coords).map(|b| (i, b))).collect();
    for (x, (i, a)) in boxes.iter().enumerate() {
      for (j, b) in &boxes[x + 1..] {
        if near(a, b) {
          uf.union(*i, *j);
        }
      }
    }
  }

  let mut members: HashMap<usize, Vec<usize>> = HashMap::new();
  for i in 0..streets.len() {
    members.entry(uf.find(i)).or_default().push(i);
  }

  let mut rows = Vec::new();
  let mut component = vec![usize::MAX; streets.len()];
  let mut class = HashMap::new();
  for (&root, ways) in &members {
    let refs: Vec<i64> = ways.iter().flat_map(|&i| streets[i].way.refs.iter().copied()).collect();
    let Some(b) = bbox(&refs, coords) else { continue };
    let center = [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2];
    let Some(at) = refs
      .iter()
      .filter_map(|n| coords.get(n))
      .filter(|c| **c != UNSEEN)
      .min_by_key(|c| (i64::from(c[0] - center[0])).pow(2) + (i64::from(c[1] - center[1])).pow(2))
    else {
      continue;
    };

    for &i in ways {
      component[i] = root;
    }
    class.insert(root, ways.iter().map(|&i| streets[i].class).min().unwrap_or(3));
    let id = ways.iter().map(|&i| streets[i].way.id).min().unwrap_or_default();
    let name = streets[ways[0]].way.name.clone();
    rows.push(row(format!("w{id}"), PlaceKind::Street, name, "Calle", STREET_RANK, None, degrees(*at)));
  }

  let mut touching: Vec<(i64, usize)> = streets
    .iter()
    .enumerate()
    .filter(|(i, _)| component[*i] != usize::MAX)
    .flat_map(|(i, s)| s.way.refs.iter().map(move |&n| (n, i)))
    .map(|(n, i)| (n, component[i]))
    .collect();
  touching.sort_unstable();
  touching.dedup();

  let mut corners: HashMap<(usize, usize), Vec<(i64, Coord)>> = HashMap::new();
  for group in touching.chunk_by(|a, b| a.0 == b.0) {
    if group.len() < 2 {
      continue;
    }
    let node = group[0].0;
    let Some(&at) = coords.get(&node).filter(|c| **c != UNSEEN) else { continue };
    for (x, &(_, a)) in group.iter().enumerate() {
      for &(_, b) in &group[x + 1..] {
        let found = corners.entry((a, b)).or_default();
        if found.iter().all(|(_, c)| meters(degrees(*c), degrees(at)) > SAME_CORNER_M) {
          found.push((node, at));
        }
      }
    }
  }

  let label = |c: usize| (class[&c], &streets[members[&c][0]].way.name);
  for ((a, b), found) in corners {
    let (first, second) = if label(a) <= label(b) { (label(a).1, label(b).1) } else { (label(b).1, label(a).1) };
    for (node, at) in found {
      let name = format!("{first} y {second}");
      let mut r =
        row(format!("n{node}"), PlaceKind::Corner, name, "Esquina", CORNER_RANK, Some(second.as_str()), degrees(at));
      r.key = search_key(first);
      rows.push(r);
    }
  }
  rows
}

fn bbox(refs: &[i64], coords: &HashMap<i64, Coord>) -> Option<[i32; 4]> {
  refs.iter().filter_map(|n| coords.get(n)).filter(|c| **c != UNSEEN).fold(None, |b, c| {
    Some(match b {
      None => [c[0], c[1], c[0], c[1]],
      Some([s, w, n, e]) => [s.min(c[0]), w.min(c[1]), n.max(c[0]), e.max(c[1])],
    })
  })
}

fn near(a: &[i32; 4], b: &[i32; 4]) -> bool {
  a[0] - SAME_STREET_DEG <= b[2]
    && b[0] - SAME_STREET_DEG <= a[2]
    && a[1] - SAME_STREET_DEG <= b[3]
    && b[1] - SAME_STREET_DEG <= a[3]
}

fn degrees(c: Coord) -> [f64; 2] {
  [f64::from(c[0]) * 1e-7, f64::from(c[1]) * 1e-7]
}

fn meters(a: [f64; 2], b: [f64; 2]) -> f64 {
  let dy = (a[0] - b[0]) * 110_540.0;
  let dx = (a[1] - b[1]) * 111_320.0 * a[0].to_radians().cos();
  dx.hypot(dy)
}

/// Zones on a ~5 km grid
type Zone = (String, [f64; 2]);

struct Zones {
  cells: HashMap<(i32, i32), Vec<Zone>>,
}

impl Zones {
  const CELL_DEG: f64 = 0.05;

  fn cell([lat, lng]: [f64; 2]) -> (i32, i32) {
    ((lat / Self::CELL_DEG).floor() as i32, (lng / Self::CELL_DEG).floor() as i32)
  }

  fn new(rows: &[Row]) -> Self {
    let mut cells: HashMap<_, Vec<_>> = HashMap::new();
    for r in rows.iter().filter(|r| r.kind == PlaceKind::Zone && r.rank <= CONTEXT_MAX_RANK) {
      cells.entry(Self::cell([r.lat, r.lng])).or_default().push((r.name.clone(), [r.lat, r.lng]));
    }
    Self { cells }
  }

  fn nearest(&self, at: [f64; 2]) -> Option<&str> {
    let (y, x) = Self::cell(at);
    (y - 1..=y + 1)
      .flat_map(|cy| (x - 1..=x + 1).map(move |cx| (cy, cx)))
      .filter_map(|c| self.cells.get(&c))
      .flatten()
      .map(|(name, p)| (name.as_str(), meters(at, *p)))
      .filter(|(_, d)| *d <= CONTEXT_REACH_M)
      .min_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(name, _)| name)
  }
}

struct UnionFind(Vec<usize>);

impl UnionFind {
  fn new(n: usize) -> Self {
    Self((0..n).collect())
  }

  fn find(&mut self, mut i: usize) -> usize {
    while self.0[i] != i {
      self.0[i] = self.0[self.0[i]];
      i = self.0[i];
    }
    i
  }

  fn union(&mut self, a: usize, b: usize) {
    let (a, b) = (self.find(a), self.find(b));
    if a != b {
      self.0[a.max(b)] = a.min(b);
    }
  }
}

impl Row {
  pub fn columns(rows: &[Row]) -> Columns {
    let mut c = Columns::default();
    for r in rows {
      c.ids.push(r.id.clone());
      c.kinds.push(r.kind.as_str().to_string());
      c.names.push(r.name.clone());
      c.categories.push(r.category.to_string());
      c.contexts.push(r.context.clone());
      c.keys.push(r.key.clone());
      c.crossings.push(r.crossing.clone());
      c.ranks.push(r.rank);
      c.lngs.push(r.lng);
      c.lats.push(r.lat);
    }
    c
  }
}

#[derive(Default)]
pub struct Columns {
  pub ids: Vec<String>,
  pub kinds: Vec<String>,
  pub names: Vec<String>,
  pub categories: Vec<String>,
  pub contexts: Vec<Option<String>>,
  pub keys: Vec<String>,
  pub crossings: Vec<Option<String>>,
  pub ranks: Vec<i16>,
  pub lngs: Vec<f64>,
  pub lats: Vec<f64>,
}
