use crate::core::format::place_name;

const QUALIFIERS: [&str; 5] = [" DESDE ", " DE ", " X ", " POR ", " A "];
const PREFIXES: [&str; 2] = ["DIRECTO ", "EXPRESO "];
const ALIAS: &[(&str, &str)] = &[
  ("P. DEL ESTE", "PUNTA DEL ESTE"),
  ("P DEL ESTE", "PUNTA DEL ESTE"),
  ("PTA DEL ESTE", "PUNTA DEL ESTE"),
  ("BAL. BS. AS.", "BALNEARIO BS. AS."),
  ("BALN. BS. AS.", "BALNEARIO BS. AS."),
  ("PAN DE AZ", "PAN DE AZÚCAR"),
];

pub fn destination(raw: &str) -> String {
  let text = raw.split('(').next().unwrap_or(raw).trim().to_uppercase();
  let destination = match text.rsplit_once(" - ") {
    Some((_, destination)) => destination.trim().to_string(),
    None => {
      let cut = QUALIFIERS.iter().filter_map(|c| text.find(c)).min().unwrap_or(text.len());
      text[..cut].trim().to_string()
    }
  };
  let destination = PREFIXES.iter().find_map(|p| destination.strip_prefix(p)).unwrap_or(&destination);
  let destination = ALIAS.iter().find(|(from, _)| *from == destination).map_or(destination, |(_, to)| to);
  place_name(destination)
}

pub fn match_key(raw: &str) -> String {
  raw.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

pub fn stop(raw: &str) -> Option<(&str, &str)> {
  let (code, name) = raw.trim().split_once(' ')?;
  code.chars().all(|c| c.is_ascii_digit()).then(|| (code, name.trim()))
}

#[cfg(test)]
mod tests {
  use super::{destination, match_key, stop};

  #[test]
  fn destination_comes_from_operator_label() {
    assert_eq!(destination("P. DEL ESTE DE AG X LAV"), "Punta del Este");
    assert_eq!(destination("SAN CARLOS A VIAL."), "San Carlos");
    assert_eq!(destination("BAL. BS. AS. DESDE MALDONADO"), "Balneario Bs. As.");
    assert_eq!(destination("LA CAPUERA - PTA DEL ESTE"), "Punta del Este");
    assert_eq!(destination("P DEL ESTE - PAN DE AZ (Contro"), "Pan de Azúcar");
    assert_eq!(destination("Maldonado - Baln. Bs. As.(por"), "Balneario Bs. As.");
    assert_eq!(destination("DIRECTO SAN CARLOS VIAL"), "San Carlos Vial");
    assert_eq!(destination("LOS CARACOLES"), "Los Caracoles");
  }

  #[test]
  fn key_survives_broken_encoding() {
    assert_eq!(match_key("Hipódromo"), match_key("Hip\u{FFFD}dromo"));
    assert_eq!(match_key("Ag. Maldonado"), "AGMALDONADO");
  }

  #[test]
  fn splits_code_and_name() {
    assert_eq!(stop("139 SAN CARLOS"), Some(("139", "SAN CARLOS")));
    assert_eq!(stop("24/5 P. DEL ESTE"), None);
  }
}
