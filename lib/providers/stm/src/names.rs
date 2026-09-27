use crate::core::format::place_name;
use crate::core::provider::Platform;

pub fn route_name(raw: &str) -> String {
  let collapsed = raw.split('-').map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" - ");
  place_name(&collapsed)
}

pub fn platform(raw: &str) -> Option<Platform> {
  let words: Vec<&str> = raw.split_whitespace().collect();
  let [station @ .., anden, number] = words.as_slice() else {
    return None;
  };
  if !matches!(anden.to_lowercase().as_str(), "anden" | "andén") || !number.chars().all(|c| c.is_ascii_digit()) {
    return None;
  }
  let station = station.strip_suffix(&["-"]).unwrap_or(station);
  (!station.is_empty()).then(|| Platform {
    station: place_name(&station.join(" ")),
    name: format!("Andén {number}"),
    code: number.to_string(),
  })
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
  use super::{company_name, platform, route_name};

  #[test]
  fn names_online_companies_like_the_api() {
    assert_eq!(company_name(50), "CUTCSA");
    assert_eq!(company_name(99), "99");
  }

  #[test]
  fn splits_platforms_from_their_station() {
    let belloni = platform("Intercambiador Belloni Andén 4").expect("andén");
    assert_eq!((belloni.station.as_str(), belloni.name.as_str()), ("Intercambiador Belloni", "Andén 4"));
    let colon = platform("Terminal Colon - Anden 5").expect("andén");
    assert_eq!((colon.station.as_str(), colon.name.as_str(), colon.code.as_str()), ("Terminal Colon", "Andén 5", "5"));
    assert!(platform("Terminal Del Cerro").is_none());
    assert!(platform("Blandengues - Pando").is_none());
    assert!(platform("Anden 3").is_none());
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
