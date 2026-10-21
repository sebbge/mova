mod fleet;
mod gtfs;
mod osm;

pub fn spawn(state: &crate::Transit) {
  gtfs::spawn_scheduler(state.clone());
  osm::spawn_scheduler(state.clone());
  fleet::spawn_pollers(state);
}
