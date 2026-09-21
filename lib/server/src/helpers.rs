pub fn seconds_to_mins(s: Option<i64>) -> Option<i64> {
  s.map(|s| (s / 60).max(0))
}

/// Natural order, must match the `Stop::get` ORDER BY
pub fn cmp_line(a: &str, b: &str) -> std::cmp::Ordering {
  fn digits(s: &str) -> Option<u64> {
    let n: String = s.chars().filter(char::is_ascii_digit).collect();
    n.parse().ok()
  }
  match (digits(a), digits(b)) {
    (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.cmp(b)),
    (Some(_), None) => std::cmp::Ordering::Less,
    (None, Some(_)) => std::cmp::Ordering::Greater,
    (None, None) => a.cmp(b),
  }
}

//# Time

pub fn secs_to_midnight() -> u64 {
  use chrono::{Local, Timelike};
  let now = Local::now();
  (86_400 - now.num_seconds_from_midnight() as u64).max(1)
}

#[cfg(test)]
mod tests {
  use super::cmp_line;

  #[test]
  fn sorts_lines_naturally() {
    let mut lines = vec!["100", "2", "L1", "17", "124 SD", "BT1", "G"];
    lines.sort_by(|a, b| cmp_line(a, b));
    assert_eq!(lines, vec!["BT1", "L1", "2", "17", "100", "124 SD", "G"]);
  }
}
