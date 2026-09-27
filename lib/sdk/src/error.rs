#[derive(Debug)]
pub enum Error {
  Config(String),
  External(String),
  Timeout,
  RateLimited,
}
pub type Result<T> = std::result::Result<T, crate::Error>;

impl From<reqwest::Error> for Error {
  fn from(e: reqwest::Error) -> Self {
    if e.is_timeout() {
      return Self::Timeout;
    }
    Self::External(e.to_string())
  }
}
