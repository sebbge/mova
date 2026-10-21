import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {BusLeg, Itinerary} from '~/lib/api/types';

export function firstBus(itinerary: Itinerary): BusLeg | undefined {
  return itinerary.legs.find((leg): leg is BusLeg => leg.mode === 'bus');
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
    return leg ? this.#etas()[`${leg.from.id}|${leg.line}`] : undefined;
  }

  watch(itineraries: Itinerary[]) {
    const byStop = new Map<string, Set<string>>();
    for (const leg of itineraries.map(firstBus)) {
      if (!leg) continue;
      const lines = byStop.get(leg.from.id) ?? new Set<string>();
      lines.add(leg.line);
      byStop.set(leg.from.id, lines);
    }

    for (const [stop, lines] of byStop) {
      const last = this.#fetched.get(stop);
      if (last && Date.now() - last < LiveBoard.REFRESH_MS) continue;
      this.#fetched.set(stop, Date.now());
      api
        .stopArrivals(stop)
        .then((detail) => {
          const found: Record<string, number | null> = {};
          for (const line of lines) {
            const live = detail.lines.find((l) => l.line === line)?.arrivals.find((a) => a.live);
            found[`${stop}|${line}`] = live?.eta ?? null;
          }
          this.#setEtas((all) => ({...all, ...found}));
        })
        .catch(() => this.#fetched.delete(stop));
    }
  }
}
