use crate::fare::FareLeg;
use crate::query::{Itinerary, Leg, Mode, Place, Query};
use crate::timetable::{Timetable, Walking};
use mova_core::geo::Point;
use mova_transit::Transit;
use mova_transit::entities::service_day::ServiceDay;
use search::{Access, Journey, Search, Step};

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

mod search;

pub struct Raptor {
  transit: Transit,
  cache: RwLock<HashMap<NaiveDate, Arc<Timetable>>>,
  building: Mutex<()>,
}

impl Raptor {
  const MAX_RIDES: usize = 4;
  const CACHED_DAYS: usize = 3;

  pub fn new(transit: Transit) -> Self {
    Self { transit, cache: RwLock::default(), building: Mutex::new(()) }
  }

  pub fn spawn(self: Arc<Self>) {
    tokio::spawn(async move {
      let mut synced = self.transit.synced();
      loop {
        if let Err(e) = self.refresh(Local::now().date_naive()).await {
          tracing::warn!(error = ?e, "[planner/build] couldn't build timetable");
        }
        tokio::select! {
          changed = synced.changed() => if changed.is_err() { return },
          _ = tokio::time::sleep(until_midnight()) => {}
        }
      }
    });
  }

  pub async fn plan(&self, query: Query) -> crate::Result<Vec<Itinerary>> {
    let timetable = self.timetable(query.at.date()).await?;
    tokio::task::spawn_blocking(move || timetable.itineraries(&query))
      .await
      .map_err(|e| crate::Error::Internal(format!("search panicked: {e}")))
  }

  async fn refresh(&self, date: NaiveDate) -> crate::Result<()> {
    let _building = self.building.lock().await;
    let timetable = self.build(date).await?;
    let mut cache = self.cache.write().await;
    cache.clear();
    cache.insert(date, timetable);
    Ok(())
  }

  async fn timetable(&self, date: NaiveDate) -> crate::Result<Arc<Timetable>> {
    if let Some(t) = self.cache.read().await.get(&date) {
      return Ok(t.clone());
    }
    let Ok(_building) = self.building.try_lock() else {
      return Err(crate::Error::NotReady);
    };
    if let Some(t) = self.cache.read().await.get(&date) {
      return Ok(t.clone());
    }

    let timetable = self.build(date).await?;
    let mut cache = self.cache.write().await;
    if cache.len() >= Self::CACHED_DAYS
      && let Some(oldest) = cache.keys().min().copied()
    {
      cache.remove(&oldest);
    }
    cache.insert(date, timetable.clone());
    Ok(timetable)
  }

  async fn build(&self, date: NaiveDate) -> crate::Result<Arc<Timetable>> {
    let started = Instant::now();
    let day = ServiceDay::load(&self.transit, date).await?;
    let fares = self.transit.fare_products().await;
    let timetable = tokio::task::spawn_blocking(move || Timetable::build(day, fares))
      .await
      .map_err(|e| crate::Error::Internal(format!("timetable build panicked: {e}")))?;

    tracing::info!(
      date = %date,
      stops = timetable.stops.len(),
      trips = timetable.trips.len(),
      patterns = timetable.patterns.len(),
      transfers = timetable.transfers.iter().map(Vec::len).sum::<usize>(),
      fares = timetable.fares.products().len(),
      ms = started.elapsed().as_millis() as u64,
      "[planner/build] timetable ready"
    );
    Ok(Arc::new(timetable))
  }
}

impl Timetable {
  const REACH_M: f64 = 800.0;
  const WALK_ONLY_M: f64 = 2000.0;

  pub(crate) fn itineraries(&self, query: &Query) -> Vec<Itinerary> {
    if query.mode == Mode::Walk {
      return vec![Itinerary::walking(self, query)];
    }

    let access = self.access(query.from, true);
    let egress = self.access(query.to, false);
    let depart = query.at.num_seconds_from_midnight() as i32;

    let mut out: Vec<Itinerary> = Vec::new();
    let journeys = Search::new(self, Raptor::MAX_RIDES).run(depart, &access, &egress);
    for found in journeys.iter().filter_map(|j| Itinerary::from_journey(self, query, j)) {
      if out.last().is_none_or(|fewer| found.worth_over(fewer)) {
        out.push(found);
      }
    }

    if Walking::between(query.from, query.to).meters <= Self::WALK_ONLY_M {
      out.push(Itinerary::walking(self, query));
    }

    let kept: Vec<bool> = out.iter().map(|x| !out.iter().any(|y| y.dominates(x))).collect();
    let mut out: Vec<Itinerary> = out.into_iter().zip(kept).filter_map(|(i, keep)| keep.then_some(i)).collect();
    out.sort_by_key(|i| (i.arrival, i.fare.cents, i.transfers));
    out
  }

  fn access(&self, point: Point, from_point: bool) -> Vec<Access> {
    self
      .near(point, Self::REACH_M)
      .into_iter()
      .map(|(stop, _)| {
        let location = self.stops[stop as usize].location;
        let w = if from_point { Walking::between(point, location) } else { Walking::between(location, point) };
        Access { stop, secs: w.secs, meters: w.meters }
      })
      .collect()
  }
}

impl Itinerary {
  const TRANSFER_WORTH_S: i64 = 300;
  const EXTRA_FARE_WORTH_S: i64 = 900;

  fn dominates(&self, x: &Itinerary) -> bool {
    let priced = x.fare.complete && self.fare.complete;
    let cheaper_or_equal = !priced || self.fare.cents <= x.fare.cents;
    let no_worse = self.arrival <= x.arrival && self.transfers <= x.transfers && cheaper_or_equal;
    let better = self.arrival < x.arrival || self.transfers < x.transfers || (priced && self.fare.cents < x.fare.cents);
    no_worse && better
  }

  fn worth_over(&self, fewer: &Itinerary) -> bool {
    let pricier = self.fare.complete && fewer.fare.complete && self.fare.cents > fewer.fare.cents;
    let needed = if pricier { Self::EXTRA_FARE_WORTH_S } else { Self::TRANSFER_WORTH_S };
    self.arrival + TimeDelta::seconds(needed) <= fewer.arrival
  }

  fn walking(tt: &Timetable, query: &Query) -> Self {
    let direct = Walking::between(query.from, query.to);
    let arrival = query.at + TimeDelta::seconds(direct.secs.into());
    Self {
      departure: query.at,
      arrival,
      duration_s: direct.secs.into(),
      transfers: 0,
      walk_m: direct.meters.round() as u32,
      fare: tt.fares.price(&[], query.rider),
      legs: vec![Leg::Walk {
        from: query.from.into(),
        to: query.to.into(),
        departure: query.at,
        arrival,
        duration_s: direct.secs.into(),
        distance_m: direct.meters.round() as u32,
      }],
    }
  }

  fn from_journey(tt: &Timetable, query: &Query, journey: &Journey) -> Option<Self> {
    let midnight = tt.date.and_time(NaiveTime::MIN);
    let at = |secs: i32| midnight + TimeDelta::seconds(secs.into());
    let place = |s: u32| {
      let stop = &tt.stops[s as usize];
      Place::Stop { id: stop.id.clone(), name: stop.name.clone(), lat: stop.location.lat, lng: stop.location.lng }
    };
    let walk = |from: Place, to: Place, start: i32, secs: i32, meters: f64| Leg::Walk {
      from,
      to,
      departure: at(start),
      arrival: at(start + secs),
      duration_s: secs.into(),
      distance_m: meters.round() as u32,
    };

    let mut legs = Vec::new();
    let mut clock: Option<i32> = None;
    let mut rides = 0u32;
    let mut walked = 0.0;

    for (i, step) in journey.steps.iter().enumerate() {
      match *step {
        Step::Access(a) => {
          let Some(Step::Ride { pattern, row, board, .. }) = journey.steps.get(i + 1) else {
            return None;
          };
          let boarding = tt.patterns[*pattern as usize].time(*row as usize, *board as usize).departure;
          if a.meters >= 1.0 {
            legs.push(walk(query.from.into(), place(a.stop), boarding - a.secs, a.secs, a.meters));
            walked += a.meters;
          }
        }
        Step::Ride { pattern, row, board, alight } => {
          let p = &tt.patterns[pattern as usize];
          let trip = &tt.trips[p.trips[row as usize] as usize];
          let (on, off) = (p.time(row as usize, board as usize), p.time(row as usize, alight as usize));
          legs.push(Leg::Bus {
            provider: trip.route.provider.clone(),
            line: trip.line.clone(),
            headsign: trip.headsign.clone(),
            route_id: trip.route.clone(),
            shape_id: trip.shape_id.clone(),
            from: place(p.stops[board as usize]),
            to: place(p.stops[alight as usize]),
            departure: at(on.departure),
            arrival: at(off.arrival),
            stops: alight - board,
            interpolated: on.interpolated || off.interpolated,
            path: p.stops[board as usize..=alight as usize]
              .iter()
              .map(|&s| {
                let at = tt.stops[s as usize].location;
                [at.lng, at.lat]
              })
              .collect(),
          });
          clock = Some(off.arrival);
          rides += 1;
        }
        Step::Transfer { from, to, secs, meters } => {
          let start = clock?;
          let secs = (secs - Timetable::MIN_CHANGE_S).max(0);
          legs.push(walk(place(from), place(to), start, secs, meters));
          clock = Some(start + secs);
          walked += meters;
        }
        Step::Egress(e) => {
          let start = clock?;
          if e.meters >= 1.0 {
            legs.push(walk(place(e.stop), query.to.into(), start, e.secs, e.meters));
            walked += e.meters;
          }
        }
      }
    }

    let departure = legs.first()?.departure();
    let arrival = legs.last()?.arrival();
    let boardings: Vec<FareLeg> = legs
      .iter()
      .enumerate()
      .filter_map(|(index, leg)| match leg {
        Leg::Bus { provider, line, departure, .. } => Some(FareLeg { index, provider, line, boarding: *departure }),
        Leg::Walk { .. } => None,
      })
      .collect();
    let fare = tt.fares.price(&boardings, query.rider);
    Some(Self {
      departure,
      arrival,
      duration_s: (arrival - departure).num_seconds(),
      transfers: rides.saturating_sub(1),
      walk_m: walked.round() as u32,
      fare,
      legs,
    })
  }
}

fn until_midnight() -> Duration {
  let now: NaiveDateTime = Local::now().naive_local();
  let next = (now.date() + TimeDelta::days(1)).and_time(NaiveTime::MIN);
  (next - now).to_std().unwrap_or(Duration::from_secs(3600)) + Duration::from_secs(60)
}
