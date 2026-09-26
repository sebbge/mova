use super::long_distance::{Corridor, Trip};
use crate::names;

use chrono::{Datelike, NaiveDateTime, Timelike};
use std::collections::HashMap;

const TOLERANCE_MIN: i64 = 10;
const DAYS: [&str; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"];

struct Departure {
  company: String,
  minute: i64,
  week: [bool; 7],
  line: String,
}

pub struct Index {
  legs: HashMap<(String, String), Vec<Departure>>,
}

impl Index {
  pub fn new(trips: &[Trip], corridors: &HashMap<(String, String), Corridor>) -> Self {
    let mut legs: HashMap<(String, String), Vec<Departure>> = HashMap::new();
    for t in trips {
      let Some(corridor) = corridors.get(&(t.company_code.clone(), t.line_code.clone())) else {
        continue;
      };
      legs.entry((names::place(&t.origin), names::place(&t.destination))).or_default().push(Departure {
        company: names::company(&t.company),
        minute: i64::from(t.departure),
        week: DAYS.map(|d| t.days.iter().any(|x| x == d)),
        line: corridor.short_name.clone(),
      });
    }
    Self { legs }
  }

  pub fn line(&self, company: &str, origin: &str, destination: &str, start: NaiveDateTime) -> Option<&str> {
    let at = i64::from(start.hour() * 60 + start.minute());
    let day = start.weekday().num_days_from_monday() as usize;

    let candidates: Vec<&Departure> = self
      .legs
      .get(&(names::place(origin), names::place(destination)))?
      .iter()
      .filter(|s| s.week[day] && (s.minute - at).abs() <= TOLERANCE_MIN)
      .collect();

    let company = names::company(company);
    let own: Vec<&Departure> = candidates.iter().copied().filter(|s| s.company.starts_with(&company)).collect();
    let pool = if own.is_empty() { candidates } else { own };

    let best = pool.iter().map(|s| (s.minute - at).abs()).min()?;
    let mut top = pool.iter().filter(|s| (s.minute - at).abs() == best).map(|s| s.line.as_str());
    let first = top.next()?;
    top.all(|l| l == first).then_some(first)
  }
}
