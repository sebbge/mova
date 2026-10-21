use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref {
  pub provider: String,
  pub id: String,
}

impl Ref {
  pub fn new(provider: impl Into<String>, id: impl Into<String>) -> Self {
    Self { provider: provider.into(), id: id.into() }
  }
}

impl fmt::Display for Ref {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}-{}", self.provider, self.id)
  }
}

impl FromStr for Ref {
  type Err = String;

  fn from_str(raw: &str) -> Result<Self, Self::Err> {
    match raw.split_once('-') {
      Some((provider, id)) if !provider.is_empty() && !id.is_empty() => Ok(Self::new(provider, id)),
      _ => Err(format!("invalid ref: {raw}")),
    }
  }
}

impl Serialize for Ref {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(self)
  }
}

impl<'de> Deserialize<'de> for Ref {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::Ref;

  #[test]
  fn keeps_hyphens_inside_the_id() {
    let parsed: Ref = "dnt-15001-A".parse().expect("valid ref");
    assert_eq!(parsed, Ref::new("dnt", "15001-A"));
    assert_eq!(parsed.to_string(), "dnt-15001-A");
    assert!("stm".parse::<Ref>().is_err());
    assert!("-3336".parse::<Ref>().is_err());
  }
}
