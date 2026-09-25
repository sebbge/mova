use crate::core::format::place_name;

pub fn route_name(raw: &str) -> String {
  let collapsed = raw.split('-').map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" - ");
  place_name(&collapsed)
}

pub fn company_name(code: u32) -> String {
  match code {
    10 => "COETC".into(),
    20 => "COME".into(),
    50 => "CUTCSA".into(),
    70 => "UCOT".into(),
    _ => code.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::{company_name, route_name};

  #[test]
  fn names_online_companies_like_the_api() {
    assert_eq!(company_name(50), "CUTCSA");
    assert_eq!(company_name(99), "99");
  }

  #[test]
  fn unifies_route_separators() {
    assert_eq!(route_name("Tres Cruces -- Malvin"), "Tres Cruces - Malvin");
    assert_eq!(route_name("Portones ---Playa del Cerro/dique"), "Portones - Playa del Cerro/dique");
    assert_eq!(route_name("Casabo--puente Carrasco"), "Casabo - Puente Carrasco");
    assert_eq!(route_name("Pocitos-casabo"), "Pocitos - Casabo");
    assert_eq!(route_name("Plaza España- Malvin"), "Plaza España - Malvin");
  }
}
