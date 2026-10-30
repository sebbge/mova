import {GeolocateControl, NavigationControl, type IControl, type Map as MapLibreMap} from 'maplibre-gl';

// compass + location, each in its own container
export class MapButtons implements IControl {
  readonly #box = document.createElement('div');
  readonly #controls: IControl[] = [
    new NavigationControl({showZoom: false}),
    new GeolocateControl({trackUserLocation: true, showAccuracyCircle: true}),
  ];

  onAdd(map: MapLibreMap): HTMLElement {
    this.#box.className = 'maplibregl-ctrl maplibregl-ctrl-group';
    for (const control of this.#controls) {
      const group = control.onAdd(map);
      group.className = '';
      this.#box.append(group);
    }
    return this.#box;
  }

  onRemove(map: MapLibreMap): void {
    for (const control of this.#controls) control.onRemove(map);
    this.#box.remove();
  }
}
