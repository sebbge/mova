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

impl Encoding {
  /// Max brotli (~1.2 s), blocking
  pub fn compress(self, body: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write;

    match self {
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
}
