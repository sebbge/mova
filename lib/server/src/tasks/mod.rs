mod fleet;
mod gtfs;

pub fn spawn(state: &crate::State) {
  gtfs::spawn_scheduler(state.clone());
  fleet::spawn_pollers(state);
}
