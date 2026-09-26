use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Region {
  Uy,
  UyAr,
  UyCa,
  UyCl,
  UyCo,
  UyDu,
  UyFd,
  UyFs,
  UyLa,
  UyMa,
  UyMo,
  UyPa,
  UyRn,
  UyRo,
  UyRv,
  UySa,
  UySj,
  UySo,
  UyTa,
  UyTt,
}

impl Region {
  pub const fn id(self) -> &'static str {
    match self {
      Self::Uy => "uy",
      Self::UyAr => "uy-ar",
      Self::UyCa => "uy-ca",
      Self::UyCl => "uy-cl",
      Self::UyCo => "uy-co",
      Self::UyDu => "uy-du",
      Self::UyFd => "uy-fd",
      Self::UyFs => "uy-fs",
      Self::UyLa => "uy-la",
      Self::UyMa => "uy-ma",
      Self::UyMo => "uy-mo",
      Self::UyPa => "uy-pa",
      Self::UyRn => "uy-rn",
      Self::UyRo => "uy-ro",
      Self::UyRv => "uy-rv",
      Self::UySa => "uy-sa",
      Self::UySj => "uy-sj",
      Self::UySo => "uy-so",
      Self::UyTa => "uy-ta",
      Self::UyTt => "uy-tt",
    }
  }

  pub const fn is_national(self) -> bool {
    matches!(self, Self::Uy)
  }
}

impl fmt::Display for Region {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.id())
  }
}
