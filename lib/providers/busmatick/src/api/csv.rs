pub fn fields(line: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut cur = String::new();
  let mut quoted = false;
  let mut chars = line.chars().peekable();

  while let Some(c) = chars.next() {
    match c {
      '"' if quoted && chars.peek() == Some(&'"') => {
        cur.push('"');
        chars.next();
      }
      '"' => quoted = !quoted,
      ',' if !quoted => out.push(std::mem::take(&mut cur)),
      c => cur.push(c),
    }
  }
  out.push(cur);
  out
}
