import {createEffect, createSignal, on, onCleanup, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, BusRoute, RouteShape, Stop, StopLine} from '~/lib/api/types';
import {LineFeed} from '~/lib/line/LineFeed';
import {ArrivalsSource} from '~/lib/stop/ArrivalsSource';
import type {MapUrl} from './MapUrl';

export class MapSelection {
  static readonly PARAM = {stop: 'parada', line: 'linea', variant: 'v', direction: 'd', bus: 'bus'} as const;

  readonly #stop: Accessor<Stop | null>;
  readonly #setStop: Setter<Stop | null>;
  readonly #arrivals: Accessor<ArrivalsSource | undefined>;
  readonly #setArrivals: Setter<ArrivalsSource | undefined>;
  readonly #line: Accessor<LineFeed | undefined>;
  readonly #setLine: Setter<LineFeed | undefined>;
  readonly #shape: Accessor<RouteShape | null>;
  readonly #setShape: Setter<RouteShape | null>;
  #directionHint: string | null = null; // tapped arrival's destination

  constructor(private readonly url: MapUrl) {
    [this.#stop, this.#setStop] = createSignal<Stop | null>(null);
    [this.#arrivals, this.#setArrivals] = createSignal<ArrivalsSource>();
    [this.#line, this.#setLine] = createSignal<LineFeed>();
    [this.#shape, this.#setShape] = createSignal<RouteShape | null>(null);

    createEffect(
      on(
        () => this.stopId,
        (id) => {
          this.#setStop(null);
          this.#setArrivals(id ? new ArrivalsSource(id) : undefined);
          if (!id) return;
          const controller = new AbortController();
          api
            .stop(id, {signal: controller.signal})
            .then((s) => this.#setStop(s))
            .catch(() => {});
          onCleanup(() => controller.abort());
        }
      )
    );

    createEffect(
      on(
        () => this.lineId,
        (line) => this.#setLine(line ? new LineFeed(line) : undefined)
      )
    );

    // by line number
    createEffect(() => {
      const first = this.variants[0];
      if (this.lineId && !this.routeId && first) this.url.select({[MapSelection.PARAM.variant]: first.route_id}, true);
    });

    createEffect(
      on([() => this.routeId, () => this.requestedDirection], ([id, direction]) => {
        this.#setShape(null);
        if (!id) return;
        const controller = new AbortController();
        api
          .routeShape(id, direction, {signal: controller.signal})
          .then((s) => this.#setShape(s))
          .catch(() => {});
        onCleanup(() => controller.abort());
      })
    );

    createEffect(() => {
      const shape = this.#shape();
      const hint = this.#directionHint?.toUpperCase();
      if (!hint || !shape) return;
      this.#directionHint = null;
      const match = shape.available_directions.find((d) => d.headsign?.toUpperCase() === hint);
      if (match && match.direction_id !== shape.direction_id) {
        this.url.select({[MapSelection.PARAM.direction]: String(match.direction_id)});
      }
    });
  }

  get stopId(): string | null {
    return this.url.get(MapSelection.PARAM.stop);
  }

  get lineId(): string | null {
    return this.url.get(MapSelection.PARAM.line);
  }

  get routeId(): string | null {
    return this.url.get(MapSelection.PARAM.variant);
  }

  // without d the server decides (one-way routes)
  get requestedDirection(): number | undefined {
    const d = this.url.get(MapSelection.PARAM.direction);
    return d === null ? undefined : Number(d);
  }

  get busId(): string | null {
    return this.url.get(MapSelection.PARAM.bus);
  }

  get stop(): Stop | null {
    return this.#stop();
  }

  get arrivals(): StopLine[] {
    return this.#arrivals()?.lines ?? [];
  }

  get loadingArrivals(): boolean {
    return this.#arrivals()?.loading ?? false;
  }

  get variants(): BusRoute[] {
    return this.#line()?.variants ?? [];
  }

  get shape(): RouteShape | null {
    return this.#shape();
  }

  get buses(): Bus[] {
    return (this.lineId ? this.#line()?.buses : this.#arrivals()?.buses) ?? [];
  }

  get selectedBus(): Bus | null {
    const id = this.busId;
    return id ? (this.buses.find((b) => b.id === id) ?? null) : null;
  }

  get open(): boolean {
    return Boolean(this.stopId || this.lineId || this.selectedBus);
  }

  get origin(): string | null {
    const shape = this.#shape();
    if (!shape) return null;

    const headsign = shape.headsign?.toLowerCase();
    if (headsign) {
      const bus = this.#line()?.buses.find((b) => b.destination?.toLowerCase().startsWith(headsign));
      if (bus?.origin) return bus.origin;
    }

    return shape.available_directions.find((d) => d.direction_id !== shape.direction_id)?.headsign ?? null;
  }

  openStop(id: string) {
    this.url.select({...this.#cleared(), [MapSelection.PARAM.stop]: id});
  }

  openLine(line: string, destination?: string) {
    this.#directionHint = destination ?? null;
    this.url.select({...this.#cleared(), [MapSelection.PARAM.line]: line});
  }

  selectVariant(routeId: string, direction?: number) {
    this.url.select({
      [MapSelection.PARAM.variant]: routeId,
      [MapSelection.PARAM.direction]: direction === undefined ? null : String(direction),
    });
  }

  selectBus(bus: Bus) {
    this.url.select({[MapSelection.PARAM.bus]: bus.id});
  }

  deselectBus() {
    if (this.busId) this.url.select({[MapSelection.PARAM.bus]: null});
  }

  close() {
    if (this.busId) this.deselectBus();
    else this.url.select(this.#cleared());
  }

  #cleared(): Record<string, null> {
    return Object.fromEntries(Object.values(MapSelection.PARAM).map((key) => [key, null]));
  }
}
