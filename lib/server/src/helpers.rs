pub fn seconds_to_mins(s: Option<i64>) -> Option<i64> {
  s.map(|s| (s / 60).max(0))
}

/// Natural order, must match the `Stop::get` ORDER BY
pub fn cmp_line(a: &str, b: &str) -> std::cmp::Ordering {
  fn digits(s: &str) -> Option<u64> {
    let n: String = s.chars().filter(char::is_ascii_digit).collect();
    n.parse().ok()
  }
  match (digits(a), digits(b)) {
    (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.cmp(b)),
    (Some(_), None) => std::cmp::Ordering::Less,
    (None, Some(_)) => std::cmp::Ordering::Greater,
    (None, None) => a.cmp(b),
  }
}

//# Compression

/// Stored encodings, `Identity` is the raw body
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
  Br,
  Gzip,
  Identity,
}

impl Encoding {
  pub const ALL: [Self; 3] = [Self::Identity, Self::Gzip, Self::Br];

  pub fn suffix(self) -> &'static str {
    match self {
      Self::Br => ":br",
      Self::Gzip => ":gz",
      Self::Identity => "",
    }
  }

  pub fn header(self) -> Option<&'static str> {
    match self {
      Self::Br => Some("br"),
      Self::Gzip => Some("gzip"),
      Self::Identity => None,
    }
  }

  /// Brotli first
  pub fn best(accept: Option<&str>) -> Self {
    let accept = accept.unwrap_or_default();
    let accepts = |token: &str| {
      accept.split(',').any(|part| {
        let mut it = part.split(';');
        let name = it.next().unwrap_or_default().trim();
        let refused = it.any(|p| p.trim().replace(' ', "") == "q=0");
        name.eq_ignore_ascii_case(token) && !refused
      })
    };

    if accepts("br") {
      Self::Br
    } else if accepts("gzip") {
      Self::Gzip
    } else {
      Self::Identity
    }
  }
}

/// Max brotli (~1.2 s), blocking
pub fn compress(body: &[u8], enc: Encoding) -> std::io::Result<Vec<u8>> {
  use std::io::Write;

  match enc {
    Encoding::Identity => Ok(body.to_vec()),
    Encoding::Gzip => {
      let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
      out.write_all(body)?;
      out.finish()
    }
    Encoding::Br => {
      let params = brotli::enc::BrotliEncoderParams { quality: 11, ..Default::default() };
      let mut out = Vec::new();
      brotli::BrotliCompress(&mut std::io::Cursor::new(body), &mut out, &params)?;
      Ok(out)
    }
  }
}

//# Time

pub fn secs_to_midnight() -> u64 {
  use chrono::{Local, Timelike};
  let now = Local::now();
  (86_400 - now.num_seconds_from_midnight() as u64).max(1)
}

#[cfg(test)]
mod tests {
  use super::cmp_line;

  #[test]
  fn sorts_lines_naturally() {
    let mut lines = vec!["100", "2", "L1", "17", "124 SD", "BT1", "G"];
    lines.sort_by(|a, b| cmp_line(a, b));
    assert_eq!(lines, vec!["BT1", "L1", "2", "17", "100", "124 SD", "G"]);
  }
}
