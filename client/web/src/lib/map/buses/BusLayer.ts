import {Marker, type Map as MapLibreMap} from 'maplibre-gl';
import {createComponent, createRoot, createSignal, type Setter} from 'solid-js';
import {render} from 'solid-js/web';
import type {Bus, RouteShape} from '~/lib/api/types';
import {follows} from '~/lib/domain/buses';
import BusPin from './BusPin';

type LngLat = [number, number];

interface Entry {
  marker: Marker;
  setBus: Setter<Bus>;
  setDimmed: Setter<boolean>;
  setReceded: Setter<boolean>;
  dispose: () => void;
  from: LngLat;
  to: LngLat;
  start: number;
}

// dom markers, the pin is css
export class BusLayer {
  static readonly TWEEN_MS = 900;
  static readonly TWEEN_MAX = 600; // m, farther teleports

  readonly #entries = new Map<string, Entry>();
  readonly #instant = matchMedia('(prefers-reduced-motion: reduce)').matches;
  #frame: number | null = null;
  // one focus for every pin
  readonly #focus = createRoot(() => createSignal<string | null>(null));

  constructor(
    private readonly map: MapLibreMap,
    private readonly onSelect?: (bus: Bus) => void
  ) {}

  // far or with a route, off-route buses shrink and fade
  render(buses: Bus[], focusId: string | null = null, shape: RouteShape | null = null, far = false) {
    const seen = new Set<string>();
    const now = performance.now();
    this.#focus[1](focusId);

    for (const bus of buses) {
      const offRoute = bus.id !== focusId && shape !== null && !follows(bus, shape);
      if (focusId !== null && offRoute) continue;
      seen.add(bus.id);
      const entry = this.#entries.get(bus.id) ?? this.#add(bus);

      entry.setBus(bus); // no remount, keeps the animation
      entry.setDimmed(offRoute);
      entry.setReceded(bus.id !== focusId && (far || focusId !== null || shape !== null));
      entry.marker.getElement().style.zIndex = bus.id === focusId ? '1' : '';
      this.#moveTo(entry, [bus.location.lng, bus.location.lat], now);
    }

    for (const [id, entry] of this.#entries) {
      if (seen.has(id)) continue;
      entry.dispose();
      entry.marker.remove();
      this.#entries.delete(id);
    }

    this.#start();
  }

  destroy() {
    if (this.#frame !== null) cancelAnimationFrame(this.#frame);
    this.#frame = null;
    for (const entry of this.#entries.values()) {
      entry.dispose();
      entry.marker.remove();
    }
    this.#entries.clear();
  }

  #moveTo(entry: Entry, to: LngLat, now: number) {
    const current = entry.marker.getLngLat();
    const from: LngLat = [current.lng, current.lat];

    if (this.#instant || metersApart(from, to) > BusLayer.TWEEN_MAX) {
      entry.marker.setLngLat(to);
      entry.from = to;
      entry.to = to;
      entry.start = now - BusLayer.TWEEN_MS;
      return;
    }

    // from the current pos, never jerks back
    entry.from = from;
    entry.to = to;
    entry.start = now;
  }

  #start() {
    if (this.#frame === null) this.#frame = requestAnimationFrame(this.#tick);
  }

  #tick = () => {
    const now = performance.now();
    let moving = false;

    for (const entry of this.#entries.values()) {
      const t = Math.min(1, (now - entry.start) / BusLayer.TWEEN_MS);
      if (t >= 1) continue;

      const k = 1 - (1 - t) ** 3; // ease out
      entry.marker.setLngLat([
        entry.from[0] + (entry.to[0] - entry.from[0]) * k,
        entry.from[1] + (entry.to[1] - entry.from[1]) * k,
      ]);
      moving = true;
    }

    this.#frame = moving ? requestAnimationFrame(this.#tick) : null;
  };

  #add(initial: Bus): Entry {
    const at: LngLat = [initial.location.lng, initial.location.lat];
    const element = document.createElement('div');
    const [bus, setBus] = createSignal(initial);
    const [dimmed, setDimmed] = createSignal(false);
    const [receded, setReceded] = createSignal(false);
    const [focus] = this.#focus;
    const onSelect = this.onSelect;
    const dispose = render(
      () =>
        createComponent(BusPin, {
          get bus() {
            return bus();
          },
          get dimmed() {
            return dimmed();
          },
          get receded() {
            return receded();
          },
          get selected() {
            return focus() === bus().id;
          },
          onSelect,
        }),
      element
    );
    const marker = new Marker({element}).setLngLat(at).addTo(this.map);

    const entry = {
      marker,
      setBus,
      setDimmed,
      setReceded,
      dispose,
      from: at,
      to: at,
      start: performance.now() - BusLayer.TWEEN_MS,
    };
    this.#entries.set(initial.id, entry);
    return entry;
  }
}

function metersApart(a: LngLat, b: LngLat): number {
  const lat = ((a[1] + b[1]) / 2) * (Math.PI / 180);
  const dx = (a[0] - b[0]) * 111_320 * Math.cos(lat);
  const dy = (a[1] - b[1]) * 110_574;
  return Math.hypot(dx, dy);
}
