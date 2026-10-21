use super::*;

mod repository;

///# GET <MOVA>/places/search?q={term}&near={lat,lng}
///# GET <MOVA>/place/{id}
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Place {
  pub id: String,
  pub kind: PlaceKind,
  pub name: String,
  pub category: String,
  pub context: Option<String>,
  pub lat: f64,
  pub lng: f64,
  pub area: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum PlaceKind {
  Zone,
  Poi,
  Street,
  Corner,
}

impl PlaceKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Zone => "zone",
      Self::Poi => "poi",
      Self::Street => "street",
      Self::Corner => "corner",
    }
  }
}

const ABBREVIATIONS: &[(&str, &str)] = &[
  ("avenida", "av"),
  ("bulevar", "bv"),
  ("boulevard", "bv"),
  ("doctor", "dr"),
  ("general", "gral"),
  ("ingeniero", "ing"),
  ("presidente", "pte"),
  ("coronel", "cnel"),
  ("profesor", "prof"),
  ("camino", "cno"),
];

/// Lowercase, no accents nor punctuation, single spaces
pub fn normalize(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  for c in text.chars().flat_map(char::to_lowercase) {
    let c = match c {
      'á' | 'à' | 'ä' | 'â' => 'a',
      'é' | 'è' | 'ë' | 'ê' => 'e',
      'í' | 'ì' | 'ï' | 'î' => 'i',
      'ó' | 'ò' | 'ö' | 'ô' => 'o',
      'ú' | 'ù' | 'ü' | 'û' => 'u',
      'ñ' => 'n',
      c if c.is_alphanumeric() => c,
      _ => ' ',
    };
    if c != ' ' || !out.ends_with(' ') {
      out.push(c);
    }
  }
  out.trim().to_string()
}

pub fn search_key(name: &str) -> String {
  let mut key = normalize(name);
  let extra: Vec<&str> =
    key.split(' ').filter_map(|w| ABBREVIATIONS.iter().find(|(full, _)| *full == w).map(|(_, abbr)| *abbr)).collect();
  for abbr in extra {
    key.push(' ');
    key.push_str(abbr);
  }
  key
}

/// "rivera y soca", "rivera esq. soca", "rivera / soca" -> both streets, normalized
fn corner(term: &str) -> Option<(String, String)> {
  let lower = term.to_lowercase().replace(['&', '/'], " y ");
  let words: Vec<&str> = lower.split_whitespace().collect();
  let at = words.iter().position(|w| matches!(*w, "y" | "esq" | "esq." | "esquina"))?;
  let (a, b) = (normalize(&words[..at].join(" ")), normalize(&words[at + 1..].join(" ")));
  (!a.is_empty() && !b.is_empty()).then_some((a, b))
}
