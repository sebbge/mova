// id ISO 3166-2, slug subdivision only
export interface Region {
  id: string;
  slug: string;
  name: string;
  bounds: [[number, number], [number, number]]; // [[west, south], [east, north]]
}

export const REGIONS: readonly Region[] = [
  {
    id: 'uy-mo',
    slug: 'mo',
    name: 'Montevideo',
    bounds: [
      [-56.431, -34.938],
      [-56.017, -34.702],
    ],
  },
  {
    id: 'uy-ca',
    slug: 'ca',
    name: 'Canelones',
    bounds: [
      [-56.494, -34.88],
      [-55.393, -34.207],
    ],
  },
  {
    id: 'uy-sj',
    slug: 'sj',
    name: 'San José',
    bounds: [
      [-57.153, -34.784],
      [-56.342, -33.832],
    ],
  },
];

// starts over Montevideo
export const MAP_CAMERA = {
  center: {lat: -34.9011, lng: -56.1645},
  zoom: 13,
  minZoom: 6,
  bounds: [
    [-58.55, -35.1],
    [-53.1, -30.05],
  ] as [[number, number], [number, number]],
};

export type MapCamera = typeof MAP_CAMERA;

export const DEFAULT_REGION = REGIONS[0];

const BY_ID = new Map(REGIONS.map((r) => [r.id, r]));

export function regionById(id: string | undefined): Region | undefined {
  return id ? BY_ID.get(id) : undefined;
}

export function regionsInView(west: number, south: number, east: number, north: number): Region[] {
  return REGIONS.filter(({bounds: [[w, s], [e, n]]}) => w <= east && e >= west && s <= north && n >= south);
}
