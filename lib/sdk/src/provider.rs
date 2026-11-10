use crate::arrivals::Arrivals;
use crate::fares::Fares;
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
  pub fares: Option<Arc<dyn Fares>>,
}

impl Provider {
  pub fn new(id: &'static str, authority: Region, regions: &'static [Region]) -> Self {
    Self { id, authority, regions, feed: None, vehicles: None, arrivals: None, fares: None }
  }

  pub fn feed(self, feed: impl Feed + 'static) -> Self {
    Self { feed: Some(Arc::new(feed)), ..self }
  }

  pub fn fleet(self, fleet: impl Fleet + 'static) -> Self {
    Self { vehicles: Some(Vehicles::Fleet(Arc::new(fleet))), ..self }
  }

  pub fn lines(self, lines: impl LineBuses + 'static) -> Self {
    Self { vehicles: Some(Vehicles::Lines(Arc::new(lines))), ..self }
  }

  pub fn arrivals(self, arrivals: impl Arrivals + 'static) -> Self {
    Self { arrivals: Some(Arc::new(arrivals)), ..self }
  }

  pub fn fares(self, fares: impl Fares + 'static) -> Self {
    Self { fares: Some(Arc::new(fares)), ..self }
  }
}
