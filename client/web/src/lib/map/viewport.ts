import type {LngLatBounds, Map as MapLibreMap} from 'maplibre-gl';

export const STOPS_MAX_KM2 = 300;

const WIDE = 768; // px, sheet stops covering the map

export function viewportKm2(b: LngLatBounds): number {
  const midLat = (((b.getSouth() + b.getNorth()) / 2) * Math.PI) / 180;
  const height = (b.getNorth() - b.getSouth()) * 110_574;
  const width = (b.getEast() - b.getWest()) * 111_320 * Math.cos(midLat);
  return (width * height) / 1e6;
}

export function overlaps(a: LngLatBounds, b: LngLatBounds): boolean {
  return (
    a.getWest() <= b.getEast() &&
    a.getEast() >= b.getWest() &&
    a.getSouth() <= b.getNorth() &&
    a.getNorth() >= b.getSouth()
  );
}

// narrow, the sheet covers the lower half
export function sheetOffset(map: MapLibreMap): [number, number] {
  const {width, height} = map.getCanvas().getBoundingClientRect();
  return width >= WIDE ? [0, 0] : [0, -height * 0.22];
}

// never zooms out
export function stopsZoom(map: MapLibreMap): number {
  const zoom = map.getZoom();
  const km2 = viewportKm2(map.getBounds());
  const target = STOPS_MAX_KM2 * 0.85;
  return km2 <= target ? zoom : zoom + Math.log2(km2 / target) / 2;
}
