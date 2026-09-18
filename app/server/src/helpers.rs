pub fn sentence_case(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
  }
}

pub fn seconds_to_mins(s: Option<i64>) -> Option<i64> {
  s.map(|s| (s / 60).max(0))
}
