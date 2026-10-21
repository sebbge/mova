import type {Feature, FeatureCollection} from 'geojson';
import {api} from '~/lib/api/client';
import type {BusLeg, Itinerary, TripPlace} from '~/lib/api/types';
import {serviceOf} from '~/lib/domain/providers';
import type {Place} from './TripFeed';

type Coord = [number, number];

// straight path if it can't be cut
export async function busPath(leg: BusLeg): Promise<Coord[]> {
  if (!leg.shape_id) return leg.path;
  try {
    const shape = await api.shape(`${leg.provider}-${leg.shape_id}`);
    const coords = shape.shape?.coordinates as Coord[] | undefined;
    if (!coords || coords.length < 2) return leg.path;
    const start = nearest(coords, leg.from.lng, leg.from.lat, 0);
    const end = nearest(coords, leg.to.lng, leg.to.lat, start);
    if (end <= start) return leg.path;
    return [[leg.from.lng, leg.from.lat], ...coords.slice(start, end + 1), [leg.to.lng, leg.to.lat]];
  } catch {
    return leg.path;
  }
}

export function tripFeatures(
  itinerary: Itinerary | null,
  paths: Map<number, Coord[]>,
  a: Place | null,
  b: Place | null
): FeatureCollection {
  const features: Feature[] = [];

  itinerary?.legs.forEach((leg, i) => {
    if (leg.mode === 'walk') {
      features.push({
        type: 'Feature',
        geometry: {type: 'LineString', coordinates: [coord(leg.from), coord(leg.to)]},
        properties: {kind: 'walk'},
      });
      return;
    }
    const service = serviceOf(`${leg.provider}-`);
    features.push({
      type: 'Feature',
      geometry: {type: 'LineString', coordinates: paths.get(i) ?? leg.path},
      properties: {kind: 'bus', service},
    });
    for (const stop of [leg.from, leg.to]) {
      features.push({
        type: 'Feature',
        geometry: {type: 'Point', coordinates: coord(stop)},
        properties: {kind: 'transfer', service},
      });
    }
  });

  for (const [place, kind, label] of [
    [a, 'origin', 'A'],
    [b, 'destination', 'B'],
  ] as const) {
    if (!place) continue;
    features.push({
      type: 'Feature',
      geometry: {type: 'Point', coordinates: [place.lng, place.lat]},
      properties: {kind, label},
    });
  }

  return {type: 'FeatureCollection', features};
}

function coord(p: TripPlace): Coord {
  return [p.lng, p.lat];
}

function nearest(coords: Coord[], lng: number, lat: number, from: number): number {
  const scale = Math.cos((lat * Math.PI) / 180);
  let best = from;
  let bestD = Infinity;
  for (let i = from; i < coords.length; i++) {
    const dx = (coords[i][0] - lng) * scale;
    const dy = coords[i][1] - lat;
    const d = dx * dx + dy * dy;
    if (d < bestD) {
      bestD = d;
      best = i;
    }
  }
  return best;
}
