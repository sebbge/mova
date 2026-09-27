use mova_provider_busmatick::{BusmatickConfig, codesa, copay, mdoturismo, micro, microrivera, solantigua};
use mova_sdk::{provider::Provider, vehicles::Vehicles};

fn config(base: &str) -> BusmatickConfig {
  BusmatickConfig { base: base.to_string() }
}

async fn fleet_size(provider: Provider) -> usize {
  let Some(Vehicles::Fleet(fleet)) = provider.vehicles else { panic!("{} has no fleet", provider.id) };
  fleet.fleet().await.unwrap_or_else(|e| panic!("{}: {e:?}", provider.id)).len()
}

#[tokio::test]
#[ignore = "network"]
async fn three_instances_respond() {
  let codesa = codesa(config("http://ip.codesa.com.uy/pub")).expect("config");
  let mdoturismo = mdoturismo(config("http://microltda.ddns.net:32495/pub")).expect("config");
  let micro = micro(config("http://microltda.ddns.net:32496/pub")).expect("config");

  let (a, b, c) = tokio::join!(fleet_size(codesa), fleet_size(mdoturismo), fleet_size(micro));
  eprintln!("codesa={a} mdoturismo={b} micro={c}");
}

#[tokio::test]
#[ignore = "network"]
async fn paysandu_y_rivera_responden() {
  let copay = copay(config("https://bus.copay.com.uy:10443/pub")).expect("config");
  let rivera = microrivera(config("http://microltda.ddns.net:8705/pub")).expect("config");

  let (a, b) = tokio::join!(fleet_size(copay), fleet_size(rivera));
  eprintln!("copay={a} microrivera={b}");
}

#[tokio::test]
#[ignore = "network"]
async fn codesa_feed_builds_from_network() {
  let provider = codesa(config("http://ip.codesa.com.uy/pub")).expect("config");
  let feed = provider.feed.expect("codesa has a feed");

  let version = feed.version().await.expect("version");
  let gtfs = feed.download().await.expect("feed").gtfs;
  let parsed = gtfs_structures::Gtfs::from_reader(std::io::Cursor::new(gtfs)).expect("valid gtfs");
  eprintln!(
    "version={version} routes={} trips={} stops={}",
    parsed.routes.len(),
    parsed.trips.len(),
    parsed.stops.len()
  );
  assert!(parsed.trips.len() > 400);
}

#[tokio::test]
#[ignore = "network"]
async fn sol_antigua_fetches_feed_and_binds_buses() {
  let provider = solantigua(config("http://solantigua.ddns.net:2780/pub")).expect("config");
  let feed = provider.feed.clone().expect("solantigua has a feed");

  let version = feed.version().await.expect("version");
  let gtfs = feed.download().await.expect("feed").gtfs;
  let parsed = gtfs_structures::Gtfs::from_reader(std::io::Cursor::new(gtfs)).expect("valid gtfs");
  eprintln!(
    "version={version} routes={} trips={} stops={}",
    parsed.routes.len(),
    parsed.trips.len(),
    parsed.stops.len()
  );

  let Some(Vehicles::Fleet(fleet)) = provider.vehicles else { panic!("no fleet") };
  for bus in fleet.fleet().await.expect("fleet") {
    eprintln!("{} {} {:?}", bus.id, bus.line, bus.shape_id);
  }
}
