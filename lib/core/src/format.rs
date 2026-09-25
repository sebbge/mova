const PARTICLES: &[&str] =
  &["y", "e", "o", "u", "de", "del", "la", "las", "el", "los", "al", "a", "en", "por", "con", "para", "sin"];

pub fn sentence_case(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
  }
}

pub fn line_name(raw: &str) -> String {
  raw.trim().to_uppercase()
}

pub fn place_name(raw: &str) -> String {
  raw.split_whitespace().enumerate().map(|(i, word)| normalize_word(word, i == 0)).collect::<Vec<_>>().join(" ")
}

fn normalize_word(word: &str, first: bool) -> String {
  let lower = word.to_lowercase();
  let core = lower.trim_matches(|c: char| !c.is_alphanumeric());

  if !first && PARTICLES.contains(&core) {
    return lower;
  }

  let mut out = String::with_capacity(lower.len());
  let mut capitalized = false;
  for c in lower.chars() {
    if !capitalized && c.is_alphanumeric() {
      out.extend(c.to_uppercase());
      capitalized = true;
    } else {
      out.push(c);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::{line_name, place_name};

  #[test]
  fn line_names_are_identifiers() {
    assert_eq!(line_name("Bt1"), "BT1");
    assert_eq!(line_name("124 Sd"), "124 SD");
    assert_eq!(line_name(" 6r6 "), "6R6");
  }

  #[test]
  fn fixes_title_case() {
    assert_eq!(place_name("Coruña Y Gral Julio Amadeo Roletti"), "Coruña y Gral Julio Amadeo Roletti");
    assert_eq!(place_name("Cementerio Del Norte"), "Cementerio del Norte");
    assert_eq!(place_name("Av 18 De Julio - Plaza Cagancha"), "Av 18 de Julio - Plaza Cagancha");
  }

  #[test]
  fn fixes_uppercase() {
    assert_eq!(place_name("SAYAGO (POR CNO LECOCQ)"), "Sayago (por Cno Lecocq)");
    assert_eq!(place_name("PLAYA NORTE"), "Playa Norte");
  }

  #[test]
  fn keeps_leading_particle() {
    assert_eq!(place_name("DE LA COSTA"), "De la Costa");
  }
}
