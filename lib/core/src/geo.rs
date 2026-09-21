use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Point {
  pub lat: f64,
  pub lng: f64,
}

impl Point {
  pub fn distance_to(&self, other: &Point) -> f64 {
    const R: f64 = 6_371_000.0;
    let dlat = (other.lat - self.lat).to_radians();
    let dlng = (other.lng - self.lng).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
      + self.lat.to_radians().cos() * other.lat.to_radians().cos() * (dlng / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
  }

  pub fn bearing_to(&self, other: &Point) -> f64 {
    let (rlat1, rlat2) = (self.lat.to_radians(), other.lat.to_radians());
    let dlng = (other.lng - self.lng).to_radians();
    let y = dlng.sin() * rlat2.cos();
    let x = rlat1.cos() * rlat2.sin() - rlat1.sin() * rlat2.cos() * dlng.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
  }
}
