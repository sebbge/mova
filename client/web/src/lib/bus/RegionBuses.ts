import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {RegionBus} from '~/lib/api/types';

// regions with allBuses
export class RegionBuses {
  static readonly BUSES_MS = 10_000;

  readonly #byRegion: Accessor<Record<string, RegionBus[]>>;
  readonly #setByRegion: Setter<Record<string, RegionBus[]>>;

  constructor() {
    [this.#byRegion, this.#setByRegion] = createSignal<Record<string, RegionBus[]>>({});
  }

  get buses(): RegionBus[] {
    return Object.values(this.#byRegion()).flat();
  }

  // stable string, no polling restart on pan
  follow(ids: string): (() => void) | undefined {
    const regions = ids ? ids.split(',') : [];
    this.#setByRegion((all) => Object.fromEntries(regions.filter((id) => id in all).map((id) => [id, all[id]])));
    if (regions.length === 0) return;

    const controller = new AbortController();
    const load = () =>
      regions.forEach(async (id) => {
        try {
          const buses = await api.regionBuses(id, {signal: controller.signal});
          this.#setByRegion((all) => ({...all, [id]: buses}));
        } catch {
          // keeps the last frame
        }
      });

    load();
    const timer = setInterval(load, RegionBuses.BUSES_MS);
    return () => {
      clearInterval(timer);
      controller.abort();
    };
  }

  providerOf(busId: string): string | undefined {
    return this.buses.find((b) => b.id === busId)?.provider;
  }
}
