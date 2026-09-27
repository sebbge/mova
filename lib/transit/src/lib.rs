use mova_sdk as sdk;

mod cache;
pub mod entities;
mod error;
mod helpers;
pub mod scope;
mod tasks;

use self::cache::Cache;
pub use self::error::{Error, Result};
pub use self::helpers::{Encoding, secs_to_midnight};
use self::scope::Scope;
use self::sdk::{provider::Provider, vehicles::Vehicles};

use std::collections::BTreeMap;
use std::sync::Arc;

pub type Db = sqlx::PgPool;

pub type GtfsVersions = Arc<tokio::sync::RwLock<BTreeMap<String, String>>>;

fn versions_token(versions: &BTreeMap<String, String>) -> String {
  if versions.is_empty() {
    return "none".into();
  }
  versions.iter().map(|(p, v)| format!("{p}={v}")).collect::<Vec<_>>().join(";")
}

pub struct TransitConfig {
  pub database_url: String,
  pub redis_url: String,
  pub providers: Vec<Arc<Provider>>,
  pub rebuild_webhook: Option<String>,
}

pub struct Network {
  providers: Vec<Arc<Provider>>,
  db: Db,
  tasks_db: Db,
  cache: Cache,
  gtfs_versions: GtfsVersions,
  rebuild_webhook: Option<String>,
}

#[derive(Clone)]
pub struct Transit(Arc<Network>);

impl Transit {
  pub async fn connect(config: TransitConfig) -> Result<Self> {
    let TransitConfig { database_url, redis_url, providers, rebuild_webhook } = config;
    let sync = 2 * 3600;

    let postgres = sqlx::postgres::PgPoolOptions::new().max_connections(10).connect(&database_url).await?;
    sqlx::migrate!().run(&postgres).await.map_err(|e| Error::Config(format!("migrations: {e}")))?;

    let tasks_db = {
      use sqlx::ConnectOptions;
      let options = (*postgres.connect_options())
        .clone()
        .log_slow_statements(tracing::log::LevelFilter::Warn, std::time::Duration::from_secs(30));
      sqlx::postgres::PgPoolOptions::new().max_connections(2).connect_lazy_with(options)
    };

    let versions: Vec<(String, String)> =
      sqlx::query_as("SELECT provider, version FROM gtfs_metadata").fetch_all(&postgres).await?;
    let gtfs_versions = Arc::new(tokio::sync::RwLock::new(versions.into_iter().collect()));

    let redis = redis::Client::open(redis_url)
      .map_err(|e| Error::Config(format!("invalid REDIS_URL: {e}")))?
      .get_multiplexed_async_connection()
      .await
      .map_err(|e| Error::Config(format!("redis: {e}")))?;
    let cache = Cache::new(redis, sync);

    for provider in &providers {
      tracing::info!(provider = provider.id, authority = %provider.authority, regions = ?provider.regions, "[bootstrap] loaded");

      #[cfg(debug_assertions)]
      {
        // DEBUG: force cache cold start
        use cache::Flush;
        cache.flush_region(provider.authority).await;
      }
    }

    Ok(Self(Arc::new(Network { providers, db: postgres, tasks_db, cache, gtfs_versions, rebuild_webhook })))
  }

  pub fn spawn_tasks(&self) {
    tasks::spawn(self);
  }

  pub(crate) fn scopes(&self) -> impl Iterator<Item = Scope> + '_ {
    self.providers.iter().cloned().map(Scope::new)
  }

  pub(crate) fn handles(&self) -> Vec<Arc<Provider>> {
    self.providers.clone()
  }

  pub(crate) fn priority(&self) -> Vec<String> {
    self.providers.iter().map(|p| p.id.to_string()).collect()
  }

  pub fn resolve(&self, provider: &str) -> Result<Scope> {
    let handle = self
      .providers
      .iter()
      .find(|p| p.id == provider)
      .ok_or_else(|| Error::NotFound(format!("unknown provider: {provider}")))?;
    Ok(Scope::new(handle.clone()))
  }

  pub async fn versions_token(&self) -> String {
    versions_token(&*self.gtfs_versions.read().await)
  }

  pub fn until_sync(&self) -> u64 {
    self.cache.until_sync()
  }

  pub async fn realtime(&self) -> BTreeMap<String, bool> {
    use self::cache::BusTracking;
    use self::entities::bus::Line;

    let mut realtime = BTreeMap::new();
    for scope in self.scopes() {
      let up = match scope.vehicles() {
        Some(Vehicles::Lines(vehicles)) => vehicles.probe().await,
        Some(Vehicles::Fleet(fleet)) => {
          self.cache.tracker().view(scope.provider, Line::FLEET_STALE).await.is_some() || fleet.fleet().await.is_ok()
        }
        None => false,
      };
      realtime.insert(scope.provider.to_string(), up);
    }
    realtime
  }

  pub async fn web_rebuild(&self) {
    let Some(url) = self.rebuild_webhook.as_deref() else {
      return;
    };

    match reqwest::Client::new().post(url).send().await {
      Ok(res) if res.status().is_success() => {
        tracing::info!(status = res.status().as_u16(), "[gtfs] web rebuild")
      }
      Ok(res) => tracing::warn!(status = res.status().as_u16(), "[gtfs] webhook non-2xx"),
      Err(e) => tracing::warn!(error = %e, "[gtfs] couldn't rebuild"),
    }
  }
}

impl std::ops::Deref for Transit {
  type Target = Network;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
