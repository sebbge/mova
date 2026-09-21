import {createSignal, onCleanup, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, BusRoute} from '~/lib/api/types';

export class LineFeed {
  static readonly BUSES_MS = 10_000; // server caches positions 10 s

  readonly #variants: Accessor<BusRoute[]>;
  readonly #setVariants: Setter<BusRoute[]>;
  readonly #buses: Accessor<Bus[]>;
  readonly #setBuses: Setter<Bus[]>;

  constructor(readonly line: string) {
    [this.#variants, this.#setVariants] = createSignal<BusRoute[]>([]);
    [this.#buses, this.#setBuses] = createSignal<Bus[]>([]);

    const controller = new AbortController();
    const signal = controller.signal;

    api
      .searchRoutes(line, {signal})
      .then((found) => this.#setVariants(found.filter((r) => r.route_short_name === line)))
      .catch(() => {});

    const load = async () => {
      try {
        this.#setBuses(await api.lineBuses(line, {signal}));
      } catch {
        // keeps the last frame
      }
    };
    void load();
    const timer = setInterval(() => void load(), LineFeed.BUSES_MS);

    onCleanup(() => {
      clearInterval(timer);
      controller.abort();
    });
  }

  get variants(): BusRoute[] {
    return this.#variants();
  }

  get buses(): Bus[] {
    return this.#buses();
  }
}
