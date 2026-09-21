use crate::core::text::place_name;

/// Uppercase: realtime answers `Bt1` with the whole fleet
pub fn line_name(raw: &str) -> String {
  raw.trim().to_uppercase()
}

/// `A -- B`, `A---B`, `A- B` -> `A - B`
pub fn route_name(raw: &str) -> String {
  let collapsed = raw.split('-').map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" - ");
  place_name(&collapsed)
}

#[cfg(test)]
mod tests {
  use super::{line_name, route_name};

  #[test]
  fn unifies_route_separators() {
    assert_eq!(route_name("Tres Cruces -- Malvin"), "Tres Cruces - Malvin");
    assert_eq!(route_name("Portones ---Playa del Cerro/dique"), "Portones - Playa del Cerro/dique");
    assert_eq!(route_name("Casabo--puente Carrasco"), "Casabo - Puente Carrasco");
    assert_eq!(route_name("Pocitos-casabo"), "Pocitos - Casabo");
    assert_eq!(route_name("Plaza España- Malvin"), "Plaza España - Malvin");
  }

  #[test]
  fn line_names_are_identifiers() {
    assert_eq!(line_name("Bt1"), "BT1");
    assert_eq!(line_name("124 Sd"), "124 SD");
    assert_eq!(line_name(" 522 "), "522");
    assert_eq!(line_name("L1"), "L1");
  }
}
