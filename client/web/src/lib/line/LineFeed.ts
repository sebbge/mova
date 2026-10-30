import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, BusRoute, DayType, RouteSchedule, RouteShape} from '~/lib/api/types';
import {rawId} from '~/lib/format';

type Stop = (() => void) | undefined;

// each follow* returns its cleanup
export class LineFeed {
  static readonly BUSES_MS = 10_000; // server caches positions 10 s
  static readonly TIMES_MS = 30_000;
  static readonly SCHEDULE_MS = 60_000; // refetched to move the next departure

  readonly #variants: Accessor<BusRoute[]>;
  readonly #setVariants: Setter<BusRoute[]>;
  readonly #shape: Accessor<RouteShape | null>;
  readonly #setShape: Setter<RouteShape | null>;
  readonly #buses: Accessor<Bus[]>;
  readonly #setBuses: Setter<Bus[]>;
  readonly #times: Accessor<(number | null)[]>; // aligned with shape.stops
  readonly #setTimes: Setter<(number | null)[]>;
  readonly #schedule: Accessor<RouteSchedule | null>;
  readonly #setSchedule: Setter<RouteSchedule | null>;
  readonly #scheduleFailed: Accessor<boolean>;
  readonly #setScheduleFailed: Setter<boolean>;

  constructor() {
    [this.#times, this.#setTimes] = createSignal<(number | null)[]>([]);
    [this.#schedule, this.#setSchedule] = createSignal<RouteSchedule | null>(null);
    [this.#scheduleFailed, this.#setScheduleFailed] = createSignal(false);
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

  get times(): (number | null)[] {
    return this.#times();
  }

  get schedule(): RouteSchedule | null {
    return this.#schedule();
  }

  get scheduleFailed(): boolean {
    return this.#scheduleFailed();
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

  // passing stopRef, or leaving the head
  followTimes(shape: RouteShape | null, stopRef: string | null): Stop {
    this.#setTimes([]);
    if (!shape) return;
    const controller = new AbortController();
    const at = {direction: shape.direction_id, shapeId: shape.shape_id, stop: stopRef ? rawId(stopRef) : null};
    const load = async () => {
      try {
        this.#setTimes((await api.routeTimes(shape.route_id, at, {signal: controller.signal})).times);
      } catch {}
    };
    void load();
    const timer = setInterval(() => void load(), LineFeed.TIMES_MS);
    return () => {
      clearInterval(timer);
      controller.abort();
    };
  }

  // today without day
  followSchedule(shape: RouteShape | null, stopRef: string | null, day: DayType | null): Stop {
    this.#setSchedule(null);
    this.#setScheduleFailed(false);
    if (!shape || !stopRef) return;
    const controller = new AbortController();
    const at = {direction: shape.direction_id, stop: rawId(stopRef), day};
    const load = async () => {
      try {
        this.#setSchedule(await api.routeSchedule(shape.route_id, at, {signal: controller.signal}));
        this.#setScheduleFailed(false);
      } catch {
        if (!controller.signal.aborted) this.#setScheduleFailed(true);
      }
    };
    void load();
    const timer = setInterval(() => void load(), LineFeed.SCHEDULE_MS);
    return () => {
      clearInterval(timer);
      controller.abort();
    };
  }
}
