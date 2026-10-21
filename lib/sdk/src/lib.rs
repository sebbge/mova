pub mod arrivals;
pub mod fares;
pub mod feed;
pub mod format;
pub mod http;
pub mod limiter;
pub mod provider;
pub mod vehicles;

pub use mova_core::{Ref, geo, reference, region};

mod error;
pub use error::{Error, Result};

pub mod prelude {
  pub use crate::arrivals::{Arrivals, UpcomingArrival, UpcomingByLine};
  pub use crate::fares::{Category, FarePrice, FareProduct, FareScope, Fares, Payment};
  pub use crate::feed::{Feed, LinesByStop, Platform, Static, StopName};
  pub use crate::geo::Point;
  pub use crate::provider::Provider;
  pub use crate::region::Region;
  pub use crate::vehicles::{BusEntry, BusesByLine, Fleet, LineBuses, StopRef, Vehicles};
  pub use crate::{Error, Result, format, http};
}
