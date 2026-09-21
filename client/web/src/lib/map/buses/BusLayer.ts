import {Marker, type Map as MapLibreMap} from 'maplibre-gl';
import {createComponent, createSignal, type Setter} from 'solid-js';
import {render} from 'solid-js/web';
import type {Bus} from '~/lib/api/types';
import BusPin from './BusPin';

interface Entry {
  marker: Marker;
  setBus: Setter<Bus>;
  dispose: () => void;
}

// dom markers, the pin is css
export class BusLayer {
  readonly #entries = new Map<string, Entry>();

  constructor(
    private readonly map: MapLibreMap,
    private readonly onSelect?: (bus: Bus) => void
  ) {}

  render(buses: Bus[]) {
    const seen = new Set<string>();

    for (const bus of buses) {
      seen.add(bus.id);
      const entry = this.#entries.get(bus.id);
      if (!entry) {
        this.#add(bus);
        continue;
      }
      entry.marker.setLngLat([bus.location.lng, bus.location.lat]);
      entry.setBus(bus);
    }

    for (const [id, entry] of this.#entries) {
      if (seen.has(id)) continue;
      this.#remove(entry);
      this.#entries.delete(id);
    }
  }

  destroy() {
    for (const entry of this.#entries.values()) this.#remove(entry);
    this.#entries.clear();
  }

  #add(initial: Bus) {
    const element = document.createElement('div');
    const [bus, setBus] = createSignal(initial);
    const onSelect = this.onSelect;
    const dispose = render(
      () =>
        createComponent(BusPin, {
          get bus() {
            return bus();
          },
          onSelect,
        }),
      element
    );
    const marker = new Marker({element}).setLngLat([initial.location.lng, initial.location.lat]).addTo(this.map);

    this.#entries.set(initial.id, {marker, setBus, dispose});
  }

  #remove(entry: Entry) {
    entry.dispose();
    entry.marker.remove();
  }
}
