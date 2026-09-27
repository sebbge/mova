#[derive(Debug)]
pub enum Error {
  Config(String),
  Database(sqlx::Error),
  External(String),
  Timeout,
  RateLimited,
  NotFound(String),
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<sqlx::Error> for Error {
  fn from(e: sqlx::Error) -> Self {
    Self::Database(e)
  }
}

impl From<crate::sdk::Error> for Error {
  fn from(e: crate::sdk::Error) -> Self {
    use crate::sdk::Error as Sdk;
    match e {
      Sdk::Config(msg) => Self::Config(msg),
      Sdk::External(msg) => Self::External(msg),
      Sdk::Timeout => Self::Timeout,
      Sdk::RateLimited => Self::RateLimited,
    }
  }
}
