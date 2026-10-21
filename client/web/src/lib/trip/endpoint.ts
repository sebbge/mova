export type Endpoint =
  | {kind: 'here'}
  | {kind: 'point'; lat: number; lng: number}
  | {kind: 'stop'; ref: string}
  | {kind: 'place'; id: string};

const COORDS = /^(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)$/;
const PLACE = /^[nwr]\d+$/; // osm node, way or relation

export function parseEndpoint(raw: string | null): Endpoint | null {
  if (!raw) return null;
  if (raw === 'aqui') return {kind: 'here'};
  const coords = COORDS.exec(raw);
  if (coords) return {kind: 'point', lat: Number(coords[1]), lng: Number(coords[2])};
  if (PLACE.test(raw)) return {kind: 'place', id: raw};
  return raw.includes('-') ? {kind: 'stop', ref: raw} : null;
}

export function endpointParam(endpoint: Endpoint): string {
  switch (endpoint.kind) {
    case 'here':
      return 'aqui';
    case 'point':
      return `${endpoint.lat.toFixed(5)},${endpoint.lng.toFixed(5)}`;
    case 'stop':
      return endpoint.ref;
    case 'place':
      return endpoint.id;
  }
}
