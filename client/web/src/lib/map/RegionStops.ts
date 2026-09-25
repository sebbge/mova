import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {StopItem} from '~/lib/api/types';
import type {Region} from '~/lib/domain/regions';

// per region, once each
export class RegionStops {
  readonly #stops: Accessor<StopItem[]>;
  readonly #setStops: Setter<StopItem[]>;
  readonly #loaded = new Set<string>();

  constructor() {
    [this.#stops, this.#setStops] = createSignal<StopItem[]>([]);
  }

  get all(): StopItem[] {
    return this.#stops();
  }

  find(id: string): StopItem | undefined {
    return this.#stops().find((s) => s.id === id);
  }

  async load(regions: Region[]) {
    const pending = regions.filter((r) => !this.#loaded.has(r.id));
    if (pending.length === 0) return;
    for (const r of pending) this.#loaded.add(r.id);

    const lists = await Promise.all(
      pending.map((r) =>
        api.stops(r.id).catch((e) => {
          this.#loaded.delete(r.id);
          console.error('[bondi/map] could not load stops for', r.id, e);
          return [] as StopItem[];
        })
      )
    );

    this.#setStops((all) => [...all, ...lists.flat()]);
  }
}
