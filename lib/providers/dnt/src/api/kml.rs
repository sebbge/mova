pub struct Shape {
  pub variant: String,
  pub direction: String,
  pub destination: String,
  pub coords: Vec<(f64, f64)>,
}

pub fn parse(bytes: &[u8]) -> Vec<Shape> {
  let text = String::from_utf8_lossy(bytes);
  let mut out = Vec::new();

  for block in text.split("<Placemark>").skip(1) {
    let Some(block) = block.split("</Placemark>").next() else {
      continue;
    };

    let variant = field(block, "Variante").unwrap_or_default();
    if variant.is_empty() {
      continue;
    }

    let coords = match between(block, "<coordinates>", "</coordinates>") {
      Some(raw) => coordinates(raw),
      None => continue,
    };
    if coords.len() < 2 {
      continue;
    }

    out.push(Shape {
      variant,
      direction: field(block, "Sentido").unwrap_or_default(),
      destination: field(block, "Destino").unwrap_or_default(),
      coords,
    });
  }

  out
}

fn field(block: &str, name: &str) -> Option<String> {
  let open = format!("name=\"{name}\">");
  between(block, &open, "<").map(|v| v.trim().to_string())
}

fn between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
  let start = haystack.find(open)? + open.len();
  let rest = &haystack[start..];
  let end = rest.find(close)?;
  Some(&rest[..end])
}

fn coordinates(raw: &str) -> Vec<(f64, f64)> {
  raw
    .split_whitespace()
    .filter_map(|point| {
      let mut parts = point.split(',');
      let lng: f64 = parts.next()?.parse().ok()?;
      let lat: f64 = parts.next()?.parse().ok()?;
      (lng != 0.0 && lat != 0.0).then_some((lng, lat))
    })
    .collect()
}
