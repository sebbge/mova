use crate::sdk::format::place_name;

const COMPANIES: &[(&str, &str)] = &[("COLEXPRESS", "COLONIAEXPRESS")];
const PLACES: &[(&str, &str)] = &[("BSASPORCOLONIA", "COLONIACOMP")];

pub fn match_key(raw: &str) -> String {
  raw
    .chars()
    .map(|c| match c {
      'á' | 'Á' => 'A',
      'é' | 'É' => 'E',
      'í' | 'Í' => 'I',
      'ó' | 'Ó' => 'O',
      'ú' | 'Ú' | 'ü' | 'Ü' => 'U',
      'ñ' | 'Ñ' => 'N',
      c => c.to_ascii_uppercase(),
    })
    .filter(char::is_ascii_alphanumeric)
    .collect()
}

pub fn company(raw: &str) -> String {
  alias(match_key(raw), COMPANIES)
}

pub fn place(raw: &str) -> String {
  alias(match_key(raw), PLACES)
}

fn alias(key: String, table: &[(&str, &str)]) -> String {
  table.iter().find(|(from, _)| *from == key).map_or(key, |(_, to)| to.to_string())
}

/// `codigoEmpresa` in IM/STM corresponds to`agency_id` in DNT GTFS.
pub fn company_name(code: i64) -> String {
  match code {
    10 => "COETC".into(),
    13 => "CASANOVA".into(),
    18 => "COPSA".into(),
    20 => "COME".into(),
    29 => "CITA".into(),
    35 => "TALA-PANDO-MONTEVIDEO".into(),
    50 => "CUTCSA".into(),
    70 => "UCOT".into(),
    _ => code.to_string(),
  }
}

pub fn route_name(raw: &str) -> String {
  let stripped = raw.split('(').next().unwrap_or(raw);
  let parts: Vec<&str> = stripped.split('/').map(str::trim).filter(|p| !p.is_empty()).collect();
  place_name(&parts.join(" - "))
}

#[cfg(test)]
mod tests {
  use super::{company, place, route_name};

  #[test]
  fn reconciles_seguitubus_spelling() {
    assert_eq!(company("COL. EXPRESS"), company("COLONIA EXPRESS"));
    assert_eq!(place("BS.AS. POR COLONIA"), place("COLONIA(Comp.)"));
    assert_eq!(place("Paysandú"), place("PAYSANDU"));
  }

  #[test]
  fn trims_the_via() {
    assert_eq!(route_name("MDEO./SAUCE (x 6,33,17m)"), "Mdeo. - Sauce");
    assert_eq!(route_name("PLAYA PASCUAL/MDEO."), "Playa Pascual - Mdeo.");
  }
}
