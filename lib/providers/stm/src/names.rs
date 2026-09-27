use crate::sdk::feed::{Platform, StopName};
use crate::sdk::format::place_name;

pub fn route_name(raw: &str) -> String {
  let collapsed = raw.split('-').map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" - ");
  place_name(&collapsed)
}

pub fn stop_name(raw: &str) -> StopName {
  platform(raw).unwrap_or_else(|| StopName { name: place_name(raw), platform: None })
}

fn platform(raw: &str) -> Option<StopName> {
  let words: Vec<&str> = raw.split_whitespace().collect();
  let [station @ .., anden, number] = words.as_slice() else {
    return None;
  };
  if !matches!(anden.to_lowercase().as_str(), "anden" | "andén") || !number.chars().all(|c| c.is_ascii_digit()) {
    return None;
  }
  let station = station.strip_suffix(&["-"]).unwrap_or(station);
  (!station.is_empty()).then(|| StopName {
    name: format!("Andén {number}"),
    platform: Some(Platform { station: place_name(&station.join(" ")), code: number.to_string() }),
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
  use super::{company_name, route_name, stop_name};

  #[test]
  fn names_online_companies_like_the_api() {
    assert_eq!(company_name(50), "CUTCSA");
    assert_eq!(company_name(99), "99");
  }

  #[test]
  fn splits_platforms_from_their_station() {
    let belloni = stop_name("Intercambiador Belloni Andén 4");
    let station = belloni.platform.expect("andén").station;
    assert_eq!((station.as_str(), belloni.name.as_str()), ("Intercambiador Belloni", "Andén 4"));
    let colon = stop_name("Terminal Colon - Anden 5");
    let platform = colon.platform.expect("andén");
    assert_eq!(
      (platform.station.as_str(), colon.name.as_str(), platform.code.as_str()),
      ("Terminal Colon", "Andén 5", "5")
    );
    assert!(stop_name("Terminal Del Cerro").platform.is_none());
    assert!(stop_name("Blandengues - Pando").platform.is_none());
    let bare = stop_name("Anden 3");
    assert!(bare.platform.is_none());
    assert_eq!(bare.name, "Anden 3");
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
