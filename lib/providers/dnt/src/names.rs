use crate::core::format::place_name;

pub fn route_name(raw: &str) -> String {
  let stripped = raw.split('(').next().unwrap_or(raw);
  let parts: Vec<&str> = stripped.split('/').map(str::trim).filter(|p| !p.is_empty()).collect();
  place_name(&parts.join(" - "))
}

#[cfg(test)]
mod tests {
  use super::route_name;

  #[test]
  fn trims_the_via() {
    assert_eq!(route_name("MDEO./SAUCE (x 6,33,17m)"), "Mdeo. - Sauce");
    assert_eq!(route_name("PLAYA PASCUAL/MDEO."), "Playa Pascual - Mdeo.");
  }
}
