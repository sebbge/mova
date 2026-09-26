import type {ScheduledLine, StopLine, StopSchedule} from '~/lib/api/types';

const GRACE = 60; // s
const PER_LINE = 3;

export function secondsSinceMidnight(now = new Date()): number {
  return now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();
}

// same shape as live arrivals
export function scheduleToLines(schedule: StopSchedule, now = new Date()): StopLine[] {
  const elapsed = secondsSinceMidnight(now);

  return schedule.lines
    .map((line) => ({
      line: line.line,
      destination: line.destination ?? undefined,
      company: '',
      arrivals: nextDepartures(line, elapsed).map((eta) => ({
        eta,
        live: false,
        zonal: line.zonal,
        interpolated: line.interpolated,
      })),
    }))
    .filter((line) => line.arrivals.length > 0)
    .sort((a, b) => (a.arrivals[0].eta ?? 0) - (b.arrivals[0].eta ?? 0));
}

function nextDepartures(line: ScheduledLine, elapsed: number): number[] {
  const upcoming: number[] = [];
  for (const departure of line.departures) {
    if (departure < elapsed - GRACE) continue;
    upcoming.push(Math.max(0, Math.round((departure - elapsed) / 60)));
    if (upcoming.length === PER_LINE) break;
  }
  return upcoming;
}
