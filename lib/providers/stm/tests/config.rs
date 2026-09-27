#![allow(clippy::expect_used, clippy::unwrap_used)]

use citymob_nexus_core::Error;
use citymob_provider_stm::{StmConfig, provider};

fn config() -> StmConfig {
  StmConfig {
    api_url: "https://api.example/transportepublico".into(),
    auth_url: "https://auth.example/token".into(),
    client_id: "id".into(),
    client_secret: "secret".into(),
  }
}

#[test]
fn builds_from_the_config_it_receives() {
  assert!(provider(config()).is_ok());
}

#[test]
fn rejects_an_empty_credential_naming_it() {
  let outcome = provider(StmConfig { client_secret: "  ".into(), ..config() });

  match outcome.err() {
    Some(Error::Config(msg)) => {
      assert!(msg.contains("IM_CLIENT_SECRET"), "the error should name the variable: {msg}")
    }
    other => panic!("it should fail with Config, not {other:?}"),
  }
}
