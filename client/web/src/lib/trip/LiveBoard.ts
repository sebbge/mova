import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {BusLeg, Itinerary, StopLine} from '~/lib/api/types';

export function firstBus(itinerary: Itinerary): BusLeg | undefined {
  return itinerary.legs.find((leg): leg is BusLeg => leg.mode === 'bus');
}

const norm = (s: string | null | undefined) => s?.trim().toUpperCase() ?? '';
const key = (leg: BusLeg) => `${leg.from.id}|${leg.line}|${norm(leg.headsign)}`;

// the leg's destination row counts
function serves(row: StopLine, leg: BusLeg): boolean {
  return row.line === leg.line && (!leg.headsign || norm(row.destination) === norm(leg.headsign));
}

// one arrivals call per boarding stop
export class LiveBoard {
  static readonly REFRESH_MS = 30_000;

  readonly #etas: Accessor<Record<string, number | null>>;
  readonly #setEtas: Setter<Record<string, number | null>>;
  readonly #fetched = new Map<string, number>();

  constructor() {
    [this.#etas, this.#setEtas] = createSignal<Record<string, number | null>>({});
  }

  eta(leg: BusLeg | undefined): number | null | undefined {
    return leg ? this.#etas()[key(leg)] : undefined;
  }

  watch(itineraries: Itinerary[]) {
    const byStop = new Map<string, BusLeg[]>();
    for (const leg of itineraries.map(firstBus)) {
      if (leg) byStop.set(leg.from.id, [...(byStop.get(leg.from.id) ?? []), leg]);
    }

    for (const [stop, legs] of byStop) {
      const last = this.#fetched.get(stop);
      if (last && Date.now() - last < LiveBoard.REFRESH_MS) continue;
      this.#fetched.set(stop, Date.now());
      api
        .stopArrivals(stop)
        .then((detail) => {
          const found: Record<string, number | null> = {};
          for (const leg of legs) {
            const live = detail.lines.find((l) => serves(l, leg))?.arrivals.find((a) => a.live);
            found[key(leg)] = live?.eta ?? null;
          }
          this.#setEtas((all) => ({...all, ...found}));
        })
        .catch(() => this.#fetched.delete(stop));
    }
  }
}
