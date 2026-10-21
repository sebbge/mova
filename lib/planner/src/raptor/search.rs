use crate::timetable::Timetable;

use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug)]
pub struct Access {
  pub stop: u32,
  pub secs: i32,
  pub meters: f64,
}

#[derive(Clone, Copy, Debug)]
enum Label {
  Access { secs: i32, meters: f64 },
  Ride { pattern: u32, row: u32, board: u32, alight: u32 },
  Transfer { from: u32, secs: i32, meters: f64 },
}

#[derive(Clone, Copy, Debug)]
pub enum Step {
  Access(Access),
  Ride { pattern: u32, row: u32, board: u32, alight: u32 },
  Transfer { from: u32, to: u32, secs: i32, meters: f64 },
  Egress(Access),
}

#[derive(Debug)]
pub struct Journey {
  pub steps: Vec<Step>,
}

pub struct Search<'a> {
  tt: &'a Timetable,
  arrival: Vec<Vec<i32>>,
  labels: Vec<Vec<Option<(usize, Label)>>>,
  best: Vec<i32>,
  marked: Vec<bool>,
}

impl<'a> Search<'a> {
  const UNREACHED: i32 = i32::MAX;

  pub fn new(tt: &'a Timetable, max_rides: usize) -> Self {
    let n = tt.stops.len();
    Self {
      tt,
      arrival: vec![vec![Self::UNREACHED; n]; max_rides + 1],
      labels: vec![vec![None; n]; max_rides + 1],
      best: vec![Self::UNREACHED; n],
      marked: vec![false; n],
    }
  }

  pub fn run(mut self, depart: i32, access: &[Access], egress: &[Access]) -> Vec<Journey> {
    let tt = self.tt;
    let mut frontier = Vec::new();
    for a in access {
      let s = a.stop as usize;
      let at = depart.saturating_add(a.secs);
      if at < self.arrival[0][s] {
        self.arrival[0][s] = at;
        self.best[s] = at;
        self.labels[0][s] = Some((0, Label::Access { secs: a.secs, meters: a.meters }));
        self.mark(s, &mut frontier);
      }
    }

    let mut target = Self::UNREACHED;
    let mut journeys = Vec::new();
    let mut queue: FxHashMap<u32, u32> = FxHashMap::default();
    let (mut ridden, mut walked) = (Vec::new(), Vec::new());

    for k in 1..self.arrival.len() {
      if frontier.is_empty() {
        break;
      }
      let (done, next) = self.arrival.split_at_mut(k);
      next[0].clone_from(&done[k - 1]);
      let (done, next) = self.labels.split_at_mut(k);
      next[0].clone_from(&done[k - 1]);

      queue.clear();
      for &s in &frontier {
        self.marked[s] = false;
        for &(p, pos) in &tt.stop_patterns[s] {
          queue.entry(p).and_modify(|e| *e = (*e).min(pos)).or_insert(pos);
        }
      }
      frontier.clear();

      ridden.clear();
      for (&p, &start) in &queue {
        let pattern = &tt.patterns[p as usize];
        let mut current: Option<(usize, usize)> = None;
        for pos in start as usize..pattern.stops.len() {
          let s = pattern.stops[pos] as usize;

          if let Some((row, board)) = current {
            let at = pattern.time(row, pos).arrival;
            if at < self.best[s].min(target) {
              self.arrival[k][s] = at;
              self.best[s] = at;
              let ride = Label::Ride { pattern: p, row: row as u32, board: board as u32, alight: pos as u32 };
              self.labels[k][s] = Some((k, ride));
              self.mark(s, &mut ridden);
            }
          }

          let previous = self.arrival[k - 1][s];
          if previous == Self::UNREACHED {
            continue;
          }
          let change = match self.labels[k - 1][s] {
            Some((_, Label::Ride { .. })) => Timetable::MIN_CHANGE_S,
            _ => 0,
          };
          let ready = previous.saturating_add(change);
          if current.is_some_and(|(row, _)| pattern.time(row, pos).departure < ready) {
            continue;
          }
          if let Some(row) = pattern.earliest(pos, ready)
            && current.is_none_or(|(cur, _)| row < cur)
          {
            current = Some((row, pos));
          }
        }
      }

      walked.clear();
      for &s in &ridden {
        let from = self.arrival[k][s];
        for tr in &tt.transfers[s] {
          let to = tr.to as usize;
          let at = from.saturating_add(tr.secs);
          if at < self.best[to].min(target) {
            self.arrival[k][to] = at;
            self.best[to] = at;
            self.labels[k][to] = Some((k, Label::Transfer { from: s as u32, secs: tr.secs, meters: tr.meters }));
            self.mark(to, &mut walked);
          }
        }
      }
      frontier.extend(&ridden);
      frontier.extend(&walked);

      let mut reached: Option<(i32, Access)> = None;
      for e in egress {
        let s = e.stop as usize;
        if self.arrival[k][s] == Self::UNREACHED || !matches!(self.labels[k][s], Some((r, _)) if r >= 1) {
          continue;
        }
        let at = self.arrival[k][s].saturating_add(e.secs);
        if at < target && reached.is_none_or(|(best, _)| at < best) {
          reached = Some((at, *e));
        }
      }
      if let Some((at, e)) = reached {
        target = at;
        if let Some(steps) = self.unwind(k, e) {
          journeys.push(Journey { steps });
        }
      }
    }

    journeys
  }

  fn mark(&mut self, s: usize, into: &mut Vec<usize>) {
    if !std::mem::replace(&mut self.marked[s], true) {
      into.push(s);
    }
  }

  fn unwind(&self, round: usize, egress: Access) -> Option<Vec<Step>> {
    let mut steps = vec![Step::Egress(egress)];
    let (mut round, mut s) = (round, egress.stop as usize);
    loop {
      if steps.len() > 4 * self.labels.len() + 2 {
        return None;
      }
      let (set, label) = self.labels.get(round)?.get(s).copied().flatten()?;
      match label {
        Label::Ride { pattern, row, board, alight } => {
          steps.push(Step::Ride { pattern, row, board, alight });
          s = self.tt.patterns[pattern as usize].stops[board as usize] as usize;
          round = set.checked_sub(1)?;
        }
        Label::Transfer { from, secs, meters } => {
          steps.push(Step::Transfer { from, to: s as u32, secs, meters });
          s = from as usize;
          round = set;
        }
        Label::Access { secs, meters } => {
          steps.push(Step::Access(Access { stop: s as u32, secs, meters }));
          break;
        }
      }
    }
    steps.reverse();
    Some(steps)
  }
}
