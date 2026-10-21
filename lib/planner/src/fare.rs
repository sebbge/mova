use mova_sdk::fares::{Category, FareProduct, Payment};

use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default)]
pub struct Rider {
  pub payment: Payment,
  pub category: Category,
}

pub struct FareLeg<'a> {
  pub index: usize,
  pub provider: &'a str,
  pub line: &'a str,
  pub boarding: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct Fare {
  pub currency: Option<String>,
  pub total: Option<f64>,
  pub complete: bool,
  pub tickets: Vec<Ticket>,
  #[serde(skip)]
  pub cents: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ticket {
  pub product: String,
  pub name: String,
  pub price: f64,
  pub legs: Vec<usize>,
}

#[derive(Default)]
pub struct FareCatalog {
  products: Vec<FareProduct>,
}

#[derive(Clone, Copy)]
struct Cover {
  unpriced: usize,
  cents: i64,
  from: usize,
  product: Option<usize>,
}

impl FareCatalog {
  pub fn new(products: Vec<FareProduct>) -> Self {
    Self { products }
  }

  pub fn products(&self) -> &[FareProduct] {
    &self.products
  }

  pub fn price(&self, legs: &[FareLeg], rider: Rider) -> Fare {
    let n = legs.len();
    let mut best: Vec<Option<Cover>> = vec![None; n + 1];
    best[0] = Some(Cover { unpriced: 0, cents: 0, from: 0, product: None });

    let mut options: Vec<(usize, i64, Option<usize>)> = Vec::new();
    for end in 1..=n {
      for start in 0..end {
        let Some(before) = best[start] else { continue };
        let group = &legs[start..end];
        options.clear();
        options.extend(
          self
            .products
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Self::cost(p, group, rider).map(|cents| (0, cents, Some(i)))),
        );
        if group.len() == 1 {
          options.push((1, 0, None));
        }
        for &(unpriced, cents, product) in &options {
          let cover = Cover { unpriced: before.unpriced + unpriced, cents: before.cents + cents, from: start, product };
          if best[end].is_none_or(|b| (cover.unpriced, cover.cents) < (b.unpriced, b.cents)) {
            best[end] = Some(cover);
          }
        }
      }
    }

    let mut tickets = Vec::new();
    let mut end = n;
    let mut complete = true;
    while end > 0 {
      let Some(cover) = best[end] else { break };
      match cover.product {
        Some(i) => {
          let product = &self.products[i];
          let cents = Self::cost(product, &legs[cover.from..end], rider).unwrap_or_default();
          tickets.push((product, cents, legs[cover.from..end].iter().map(|l| l.index).collect::<Vec<_>>()));
        }
        None => complete = false,
      }
      end = cover.from;
    }
    tickets.reverse();

    let cents = best[n].map_or(0, |b| b.cents);
    Fare {
      currency: tickets.first().map(|(p, _, _)| p.currency.clone()),
      total: complete.then(|| Fare::pesos(cents)),
      complete,
      tickets: tickets
        .into_iter()
        .map(|(p, cents, legs)| Ticket { product: p.id.clone(), name: p.name.clone(), price: Fare::pesos(cents), legs })
        .collect(),
      cents,
    }
  }

  fn cost(product: &FareProduct, group: &[FareLeg], rider: Rider) -> Option<i64> {
    let first = group.first()?;
    let fits = group.iter().all(|l| product.scope.covers(l.provider, l.line))
      && product.max_rides.is_none_or(|max| group.len() <= usize::from(max))
      && product
        .window_min
        .is_none_or(|window| group.iter().all(|l| (l.boarding - first.boarding).num_minutes() <= i64::from(window)))
      && product.scope.requires.iter().all(|r| group.iter().any(|l| l.provider == r));
    if !fits {
      return None;
    }
    product.price(rider.payment, rider.category)
  }
}

impl Fare {
  fn pesos(cents: i64) -> f64 {
    cents as f64 / 100.0
  }
}

#[cfg(test)]
mod tests {
  use super::{Fare, FareCatalog, FareLeg, Rider};
  use mova_sdk::fares::{Category, FareProduct, Payment};

  use chrono::{NaiveDate, NaiveDateTime};

  const PRODUCTS: &str = r#"[
    { "id": "1h", "name": "1 hora", "currency": "UYU", "window_min": 60, "max_rides": 2,
      "prices": [
        { "payment": "card", "category": "common", "cents": 5200 },
        { "payment": "cash", "category": "common", "cents": 6400 },
        { "payment": "card", "category": "student_a", "cents": 2850 }
      ],
      "scope": { "providers": ["stm"], "except_lines": ["D1"] } },
    { "id": "2h", "name": "2 horas", "currency": "UYU", "window_min": 120,
      "prices": [{ "payment": "card", "category": "common", "cents": 7800 }],
      "scope": { "providers": ["stm"] } },
    { "id": "d", "name": "Diferencial", "currency": "UYU", "max_rides": 1,
      "prices": [{ "payment": "card", "category": "common", "cents": 7800 }],
      "scope": { "providers": ["stm"], "lines": ["D1"] } },
    { "id": "metro", "name": "Combinación", "currency": "UYU", "window_min": 60, "max_rides": 2,
      "prices": [{ "payment": "card", "category": "common", "cents": 8000 }],
      "scope": { "providers": ["stm", "dnt"], "requires": ["stm", "dnt"] } }
  ]"#;

  fn catalog() -> FareCatalog {
    let products: Vec<FareProduct> = serde_json::from_str(PRODUCTS).expect("valid products");
    FareCatalog::new(products)
  }

  fn at(minute: i64) -> NaiveDateTime {
    let start = NaiveDate::from_ymd_opt(2026, 9, 28).and_then(|d| d.and_hms_opt(8, 0, 0)).expect("valid time");
    start + chrono::TimeDelta::minutes(minute)
  }

  fn price(rides: &[(&str, &str, i64)], rider: Rider) -> Fare {
    let legs: Vec<FareLeg> = rides
      .iter()
      .enumerate()
      .map(|(index, &(provider, line, minute))| FareLeg { index, provider, line, boarding: at(minute) })
      .collect();
    catalog().price(&legs, rider)
  }

  fn card(rides: &[(&str, &str, i64)]) -> Fare {
    price(rides, Rider::default())
  }

  #[test]
  fn two_rides_within_the_hour_share_a_ticket() {
    assert_eq!(card(&[("stm", "522", 0)]).total, Some(52.0));
    let pair = card(&[("stm", "522", 0), ("stm", "60", 50)]);
    assert_eq!((pair.total, pair.tickets.len()), (Some(52.0), 1));
  }

  #[test]
  fn past_the_hour_the_two_hour_ticket_is_cheaper() {
    let late = card(&[("stm", "522", 0), ("stm", "60", 70)]);
    assert_eq!((late.total, late.tickets[0].product.as_str()), (Some(78.0), "2h"));
    assert_eq!(card(&[("stm", "522", 0), ("stm", "60", 40), ("stm", "76", 100)]).total, Some(78.0));
  }

  #[test]
  fn differential_lines_take_their_own_ticket() {
    assert_eq!(card(&[("stm", "D1", 0)]).total, Some(78.0));
    assert_eq!(card(&[("stm", "D1", 0), ("stm", "522", 30)]).total, Some(78.0));
  }

  #[test]
  fn urban_and_suburban_combine() {
    assert_eq!(card(&[("stm", "522", 0), ("dnt", "700", 20)]).total, Some(80.0));
    let suburban = card(&[("dnt", "700", 0)]);
    assert!(!suburban.complete);
    assert!(suburban.tickets.is_empty());
  }

  #[test]
  fn the_rider_profile_changes_the_price() {
    let cash = Rider { payment: Payment::Cash, category: Category::Common };
    assert_eq!(price(&[("stm", "522", 0)], cash).total, Some(64.0));

    let student = Rider { payment: Payment::Card, category: Category::StudentA };
    assert_eq!(price(&[("stm", "522", 0), ("stm", "60", 20), ("stm", "76", 40)], student).total, Some(57.0));
  }
}
