#![allow(clippy::expect_used, clippy::unwrap_used)]

use citymob_core::Error;
use citymob_stm::Stm;

const ENV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.env");

#[test]
fn reads_config_from_its_own_dir() {
  let outcome = Stm::new().map(|_| ());

  if std::path::Path::new(ENV).exists() {
    assert!(outcome.is_ok(), "with {ENV} present it should build: {:?}", outcome.err());
  } else {
    match outcome.err() {
      Some(Error::Config(msg)) => assert!(msg.contains("IM_"), "the error should name the variable: {msg}"),
      other => panic!("without .env it should fail with Config, not {other:?}"),
    }
  }
}
