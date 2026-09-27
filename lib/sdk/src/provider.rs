use crate::arrivals::Arrivals;
use crate::feed::Feed;
use crate::region::Region;
use crate::vehicles::{Fleet, LineBuses, Vehicles};

use std::sync::Arc;

pub struct Provider {
  pub id: &'static str,
  pub authority: Region,
  pub regions: &'static [Region],
  pub feed: Option<Arc<dyn Feed>>,
  pub vehicles: Option<Vehicles>,
  pub arrivals: Option<Arc<dyn Arrivals>>,
}

impl Provider {
  pub fn new(id: &'static str, authority: Region, regions: &'static [Region]) -> Self {
    Self { id, authority, regions, feed: None, vehicles: None, arrivals: None }
  }

  pub fn feed(self, feed: Arc<dyn Feed>) -> Self {
    Self { feed: Some(feed), ..self }
  }

  pub fn fleet(self, fleet: Arc<dyn Fleet>) -> Self {
    Self { vehicles: Some(Vehicles::Fleet(fleet)), ..self }
  }

  pub fn lines(self, lines: Arc<dyn LineBuses>) -> Self {
    Self { vehicles: Some(Vehicles::Lines(lines)), ..self }
  }

  pub fn arrivals(self, arrivals: Arc<dyn Arrivals>) -> Self {
    Self { arrivals: Some(arrivals), ..self }
  }
}
