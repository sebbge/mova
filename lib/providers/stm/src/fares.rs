use crate::sdk::prelude::*;

const PRODUCTS: &str = include_str!("../data/fares.json");

pub struct StmFares(Vec<FareProduct>);

impl StmFares {
  pub fn load() -> Result<Self> {
    serde_json::from_str(PRODUCTS).map(Self).map_err(|e| Error::Config(format!("stm: invalid fares: {e}")))
  }
}

#[async_trait::async_trait]
impl Fares for StmFares {
  async fn products(&self) -> Result<Vec<FareProduct>> {
    Ok(self.0.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::StmFares;
  use crate::sdk::prelude::*;

  #[test]
  fn the_curated_fares_parse() {
    let StmFares(products) = StmFares::load().expect("valid fares");
    let one_hour = products.iter().find(|p| p.id == "stm-1h").expect("1 hour ticket");
    assert_eq!(one_hour.price(Payment::Card, Category::Common), Some(5200));
    assert_eq!(one_hour.price(Payment::Cash, Category::StudentA), None);
    assert!(!one_hour.scope.covers("stm", "D1"));
  }
}
