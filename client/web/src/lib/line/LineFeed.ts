import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, BusRoute, RouteShape} from '~/lib/api/types';

type Stop = (() => void) | undefined;

// each follow* returns its cleanup
export class LineFeed {
  static readonly BUSES_MS = 10_000; // server caches positions 10 s

  readonly #variants: Accessor<BusRoute[]>;
  readonly #setVariants: Setter<BusRoute[]>;
  readonly #shape: Accessor<RouteShape | null>;
  readonly #setShape: Setter<RouteShape | null>;
  readonly #buses: Accessor<Bus[]>;
  readonly #setBuses: Setter<Bus[]>;

  constructor() {
    [this.#variants, this.#setVariants] = createSignal<BusRoute[]>([]);
    [this.#shape, this.#setShape] = createSignal<RouteShape | null>(null);
    [this.#buses, this.#setBuses] = createSignal<Bus[]>([]);
  }

  get variants(): BusRoute[] {
    return this.#variants();
  }

  get shape(): RouteShape | null {
    return this.#shape();
  }

  get buses(): Bus[] {
    return this.#buses();
  }

  // route_long_name order is unreliable
  get origin(): string | null {
    const shape = this.#shape();
    if (!shape) return null;

    const headsign = shape.headsign?.toLowerCase();
    if (headsign) {
      const bus = this.#buses().find((b) => b.destination?.toLowerCase().startsWith(headsign));
      if (bus?.origin) return bus.origin;
    }

    return shape.available_directions.find((d) => d.direction_id !== shape.direction_id)?.headsign ?? null;
  }

  followVariants(line: string | null): Stop {
    if (!line) {
      this.#setVariants([]);
      return;
    }
    const controller = new AbortController();
    api
      .searchRoutes(line, {signal: controller.signal})
      .then((found) => this.#setVariants(found.filter((r) => r.route_short_name === line)))
      .catch(() => !controller.signal.aborted && this.#setVariants([]));
    return () => controller.abort();
  }

  followBuses(provider: string | null, line: string | null): Stop {
    if (!provider || !line) {
      this.#setBuses([]);
      return;
    }
    const controller = new AbortController();
    const load = async () => {
      try {
        this.#setBuses(await api.lineBuses(provider, line, {signal: controller.signal}));
      } catch {
        // keeps the last frame
      }
    };
    void load();
    const timer = setInterval(() => void load(), LineFeed.BUSES_MS);
    return () => {
      clearInterval(timer);
      controller.abort();
    };
  }

  followVariant(shapeRef: string, onMissing: () => void): Stop {
    const controller = new AbortController();
    api
      .shape(shapeRef, {signal: controller.signal})
      .then((s) => this.#setShape(s))
      .catch(() => {
        if (controller.signal.aborted) return;
        this.#setShape(null);
        onMissing();
      });
    return () => controller.abort();
  }

  followShape(routeRef: string | null, direction: number | undefined): Stop {
    if (!routeRef) {
      this.#setShape(null);
      return;
    }
    const controller = new AbortController();
    api
      .routeShape(routeRef, direction, {signal: controller.signal})
      .then((s) => this.#setShape(s))
      .catch(() => !controller.signal.aborted && this.#setShape(null));
    return () => controller.abort();
  }
}
