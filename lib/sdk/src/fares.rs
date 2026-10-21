use crate::Result;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Payment {
  #[default]
  Card,
  Cash,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
  #[default]
  Common,
  StudentA,
  StudentB,
  RetireeA,
  RetireeB,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FareProduct {
  pub id: String,
  pub name: String,
  pub currency: String,
  pub prices: Vec<FarePrice>,
  #[serde(default)]
  pub window_min: Option<u32>,
  #[serde(default)]
  pub max_rides: Option<u8>,
  pub scope: FareScope,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct FarePrice {
  pub payment: Payment,
  pub category: Category,
  pub cents: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FareScope {
  pub providers: Vec<String>,
  #[serde(default)]
  pub lines: Option<Vec<String>>,
  #[serde(default)]
  pub except_lines: Vec<String>,
  #[serde(default)]
  pub requires: Vec<String>,
}

impl FareProduct {
  pub fn price(&self, payment: Payment, category: Category) -> Option<i64> {
    self.prices.iter().find(|p| p.payment == payment && p.category == category).map(|p| p.cents)
  }
}

impl FareScope {
  pub fn covers(&self, provider: &str, line: &str) -> bool {
    self.providers.iter().any(|p| p == provider)
      && self.lines.as_ref().is_none_or(|lines| lines.iter().any(|l| l == line))
      && !self.except_lines.iter().any(|l| l == line)
  }
}

#[async_trait::async_trait]
pub trait Fares: Send + Sync {
  async fn products(&self) -> Result<Vec<FareProduct>>;
}
