use crate::fare::Rider;
use crate::query::{Itinerary, Leg, Mode, Query};
use crate::timetable::Timetable;
use mova_core::{Ref, geo::Point};
use mova_transit::entities::service_day::{DayStop, DayStopTime, DayTrip, ServiceDay};

use chrono::{NaiveDate, NaiveDateTime};

const STEP_DEG: f64 = 0.011;

fn date() -> NaiveDate {
  NaiveDate::from_ymd_opt(2026, 9, 28).expect("valid date")
}

fn at(hour: u32, minute: u32) -> NaiveDateTime {
  date().and_hms_opt(hour, minute, 0).expect("valid time")
}

fn point(km: f64) -> Point {
  Point { lat: -34.9, lng: -56.2 + km * STEP_DEG }
}

fn stop(provider: &str, id: &str, location: Point, cluster: Option<&str>) -> DayStop {
  DayStop {
    id: Ref::new(provider, id),
    name: id.to_string(),
    location,
    cluster: cluster.map_or_else(|| format!("{provider}-{id}"), str::to_string),
  }
}

fn trip(provider: &str, line: &str, times: &[(u32, i32)]) -> DayTrip {
  DayTrip {
    route: Ref::new(provider, line),
    line: line.to_string(),
    headsign: None,
    shape_id: None,
    stop_times: times
      .iter()
      .map(|&(stop, secs)| DayStopTime { stop, arrival: secs, departure: secs, interpolated: false })
      .collect(),
  }
}

fn hm(hour: i32, minute: i32) -> i32 {
  hour * 3600 + minute * 60
}

const FARES: &str = r#"[
  { "id": "1h", "name": "1 hora", "currency": "UYU", "window_min": 60, "max_rides": 2,
    "prices": [{ "payment": "card", "category": "common", "cents": 5200 }], "scope": { "providers": ["stm"] } },
  { "id": "2h", "name": "2 horas", "currency": "UYU", "window_min": 120,
    "prices": [{ "payment": "card", "category": "common", "cents": 7800 }], "scope": { "providers": ["stm"] } }
]"#;

fn plan_in(
  mode: Mode,
  stops: Vec<DayStop>,
  trips: Vec<DayTrip>,
  from: Point,
  to: Point,
  at: NaiveDateTime,
) -> Vec<Itinerary> {
  let fares = serde_json::from_str(FARES).expect("valid fares");
  let timetable = Timetable::build(ServiceDay { date: date(), stops, trips }, fares);
  timetable.itineraries(&Query { from, to, at, rider: Rider::default(), mode })
}

fn plan(stops: Vec<DayStop>, trips: Vec<DayTrip>, from: Point, to: Point, at: NaiveDateTime) -> Vec<Itinerary> {
  plan_in(Mode::Transit, stops, trips, from, to, at)
}

fn lines(itinerary: &Itinerary) -> Vec<(&str, &str)> {
  itinerary
    .legs
    .iter()
    .filter_map(|leg| match leg {
      Leg::Bus { provider, line, .. } => Some((provider.as_str(), line.as_str())),
      Leg::Walk { .. } => None,
    })
    .collect()
}

#[test]
fn rides_the_next_direct_bus() {
  let stops =
    vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(1.0), None), stop("stm", "c", point(2.0), None)];
  let trips = vec![
    trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 5)), (2, hm(8, 10))]),
    trip("stm", "1", &[(0, hm(8, 30)), (1, hm(8, 35)), (2, hm(8, 40))]),
  ];

  let found = plan(stops, trips, point(0.0), point(2.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert_eq!(lines(&found[0]), [("stm", "1")]);
  assert_eq!((found[0].departure, found[0].arrival), (at(8, 0), at(8, 10)));
  assert_eq!(found[0].transfers, 0);
}

#[test]
fn a_transfer_leaves_a_minute_to_change() {
  let stops =
    vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(3.0), None), stop("stm", "d", point(6.0), None)];
  let trips = vec![
    trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 10))]),
    trip("stm", "2", &[(1, hm(8, 10)), (2, hm(8, 20))]),
    trip("stm", "2", &[(1, hm(8, 25)), (2, hm(8, 35))]),
  ];

  let found = plan(stops, trips, point(0.0), point(6.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert_eq!(lines(&found[0]), [("stm", "1"), ("stm", "2")]);
  assert_eq!(found[0].transfers, 1);
  assert_eq!(found[0].arrival, at(8, 35));
}

#[test]
fn a_shared_shelter_connects_two_providers() {
  let shelter = point(3.0);
  let beside = Point { lat: shelter.lat + 0.0002, lng: shelter.lng };
  let stops = vec![
    stop("stm", "a", point(0.0), None),
    stop("stm", "x", shelter, None),
    stop("dnt", "y", beside, Some("stm-x")),
    stop("dnt", "d", point(6.0), None),
  ];
  let trips =
    vec![trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 10))]), trip("dnt", "700", &[(2, hm(8, 12)), (3, hm(8, 30))])];

  let found = plan(stops, trips, point(0.0), point(6.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert_eq!(lines(&found[0]), [("stm", "1"), ("dnt", "700")]);
  assert!(matches!(found[0].legs[1], Leg::Walk { .. }), "walks between the two stops of the shelter");
}

#[test]
fn after_midnight_the_trip_belongs_to_the_next_day() {
  let stops = vec![stop("stm", "a", point(0.0), None), stop("stm", "c", point(3.0), None)];
  let trips = vec![trip("stm", "1", &[(0, hm(24, 10)), (1, hm(24, 25))])];

  let found = plan(stops, trips, point(0.0), point(3.0), at(23, 55));
  assert_eq!(found.len(), 1);
  let next_day = date().succ_opt().expect("valid date");
  assert_eq!(found[0].arrival, next_day.and_hms_opt(0, 25, 0).expect("valid time"));
}

#[test]
fn an_express_that_overtakes_is_still_found() {
  let stops =
    vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(3.0), None), stop("stm", "c", point(6.0), None)];
  let trips = vec![
    trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 20)), (2, hm(8, 40))]),
    trip("stm", "1", &[(0, hm(8, 5)), (1, hm(8, 10)), (2, hm(8, 15))]),
  ];

  let found = plan(stops, trips, point(0.0), point(6.0), at(7, 55));
  assert_eq!(found[0].arrival, at(8, 15));
}

#[test]
fn walking_mode_ignores_the_buses_and_the_distance() {
  let stops = vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(5.0), None)];
  let trips = vec![trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 10))])];

  let found = plan_in(Mode::Walk, stops, trips, point(0.0), point(5.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert!(matches!(found[0].legs.as_slice(), [Leg::Walk { .. }]));
}

#[test]
fn a_bus_leg_traces_its_stops() {
  let stops =
    vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(1.0), None), stop("stm", "c", point(2.0), None)];
  let trips = vec![trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 5)), (2, hm(8, 10))])];

  let found = plan(stops, trips, point(0.0), point(2.0), at(7, 55));
  let Some(Leg::Bus { path, .. }) = found[0].legs.iter().find(|l| matches!(l, Leg::Bus { .. })) else {
    panic!("rides a bus");
  };
  assert_eq!(path.len(), 3);
  assert_eq!(path[2], [point(2.0).lng, point(2.0).lat]);
}

#[test]
fn nothing_connects_far_away_places() {
  let stops = vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(1.0), None)];
  let trips = vec![trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 5))])];

  assert!(plan(stops, trips, point(0.0), point(20.0), at(7, 55)).is_empty());
}

#[test]
fn a_transfer_that_barely_helps_is_left_out() {
  let stops =
    vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(3.0), None), stop("stm", "c", point(6.0), None)];
  let trips = vec![
    trip("stm", "1", &[(0, hm(8, 0)), (1, hm(8, 10)), (2, hm(8, 30))]),
    trip("stm", "2", &[(1, hm(8, 12)), (2, hm(8, 28))]),
  ];

  let found = plan(stops, trips, point(0.0), point(6.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert_eq!(lines(&found[0]), [("stm", "1")]);
}

fn relay(arrives: i32) -> (Vec<DayStop>, Vec<DayTrip>) {
  let stops = (0..5).map(|i| stop("stm", &format!("s{i}"), point(i as f64 * 3.0), None)).collect();
  let trips = vec![
    trip("stm", "1", &[(0, hm(8, 0)), (4, hm(9, 0))]),
    trip("stm", "2", &[(0, hm(8, 0)), (1, hm(8, 10))]),
    trip("stm", "3", &[(1, hm(8, 12)), (2, hm(8, 22))]),
    trip("stm", "4", &[(2, hm(8, 24)), (3, hm(8, 30))]),
    trip("stm", "5", &[(3, hm(8, 32)), (4, arrives)]),
  ];
  (stops, trips)
}

#[test]
fn four_buses_that_cost_another_ticket_must_save_real_time() {
  let (stops, trips) = relay(hm(8, 50));
  let found = plan(stops, trips, point(0.0), point(12.0), at(7, 55));
  assert_eq!(found.len(), 1);
  assert_eq!(lines(&found[0]), [("stm", "1")]);
  assert_eq!(found[0].fare.total, Some(52.0));

  let (stops, trips) = relay(hm(8, 40));
  let found = plan(stops, trips, point(0.0), point(12.0), at(7, 55));
  assert_eq!(found.len(), 2);
  assert_eq!(found[0].transfers, 3);
  assert_eq!(found[0].fare.total, Some(78.0));
}

#[test]
fn short_trips_are_walked() {
  let found = plan(Vec::new(), Vec::new(), point(0.0), point(0.5), at(8, 0));
  assert!(matches!(found.as_slice(), [only] if matches!(only.legs.as_slice(), [Leg::Walk { .. }])));

  let stops = vec![stop("stm", "a", point(0.0), None), stop("stm", "b", point(1.2), None)];
  let trips = vec![trip("stm", "1", &[(0, hm(8, 30)), (1, hm(8, 35))])];

  let found = plan(stops, trips, point(0.0), point(1.2), at(8, 0));
  assert_eq!(found.len(), 1);
  assert!(matches!(found[0].legs.as_slice(), [Leg::Walk { .. }]));
}
