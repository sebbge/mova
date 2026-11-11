// id ISO 3166-2, slug subdivision only
export interface Region {
  id: string;
  slug: string;
  name: string;
  bounds: [[number, number], [number, number]]; // [[west, south], [east, north]]
  allBuses?: boolean; // too few stops to pick one
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
  {
    id: 'uy-ar',
    slug: 'ar',
    name: 'Artigas',
    bounds: [
      [-57.89, -31.083],
      [-55.99, -30.085],
    ],
  },
  {
    id: 'uy-cl',
    slug: 'cl',
    name: 'Cerro Largo',
    bounds: [
      [-55.356, -33.029],
      [-53.182, -31.651],
    ],
  },
  {
    id: 'uy-co',
    slug: 'co',
    name: 'Colonia',
    allBuses: true,
    bounds: [
      [-58.432, -34.476],
      [-56.983, -33.765],
    ],
  },
  {
    id: 'uy-du',
    slug: 'du',
    name: 'Durazno',
    bounds: [
      [-57.2, -33.507],
      [-55.006, -32.409],
    ],
  },
  {
    id: 'uy-fs',
    slug: 'fs',
    name: 'Flores',
    bounds: [
      [-57.361, -33.985],
      [-56.396, -33.119],
    ],
  },
  {
    id: 'uy-fd',
    slug: 'fd',
    name: 'Florida',
    bounds: [
      [-56.536, -34.426],
      [-55.098, -33.102],
    ],
  },
  {
    id: 'uy-la',
    slug: 'la',
    name: 'Lavalleja',
    bounds: [
      [-55.625, -34.636],
      [-54.129, -33.319],
    ],
  },
  {
    id: 'uy-ma',
    slug: 'ma',
    name: 'Maldonado',
    bounds: [
      [-55.484, -35.032],
      [-54.464, -33.929],
    ],
    allBuses: true,
  },
  {
    id: 'uy-pa',
    slug: 'pa',
    name: 'Paysandú',
    allBuses: true,
    bounds: [
      [-58.207, -32.596],
      [-56.238, -31.452],
    ],
  },
  {
    id: 'uy-rn',
    slug: 'rn',
    name: 'Río Negro',
    bounds: [
      [-58.441, -33.441],
      [-56.547, -32.343],
    ],
  },
  {
    id: 'uy-rv',
    slug: 'rv',
    name: 'Rivera',
    allBuses: true,
    bounds: [
      [-56.185, -32.11],
      [-54.452, -30.833],
    ],
  },
  {
    id: 'uy-ro',
    slug: 'ro',
    name: 'Rocha',
    bounds: [
      [-54.579, -34.802],
      [-53.369, -33.151],
    ],
  },
  {
    id: 'uy-sa',
    slug: 'sa',
    name: 'Salto',
    bounds: [
      [-58.083, -31.871],
      [-56.01, -30.742],
    ],
  },
  {
    id: 'uy-so',
    slug: 'so',
    name: 'Soriano',
    bounds: [
      [-58.495, -33.956],
      [-57.089, -33.0],
    ],
  },
  {
    id: 'uy-ta',
    slug: 'ta',
    name: 'Tacuarembó',
    bounds: [
      [-56.684, -32.867],
      [-54.668, -31.231],
    ],
  },
  {
    id: 'uy-tt',
    slug: 'tt',
    name: 'Treinta y Tres',
    bounds: [
      [-55.159, -33.46],
      [-53.293, -32.697],
    ],
  },
];

// starts over Montevideo
export const MAP_CAMERA = {
  center: {lat: -34.9011, lng: -56.1645},
  zoom: 13,
  bounds: [
    [-58.55, -35.1],
    [-53.1, -30.05],
  ] as [[number, number], [number, number]],
};

export type MapCamera = typeof MAP_CAMERA & {focus?: Region['bounds']};

export const DEFAULT_REGION = REGIONS[0];

const BY_ID = new Map(REGIONS.map((r) => [r.id, r]));

// the region only picks where the camera starts
export function mapCameraFor(slug: string | null): MapCamera {
  const region = REGIONS.find((r) => r.slug === slug);
  return region ? {...MAP_CAMERA, focus: region.bounds} : MAP_CAMERA;
}

export function regionById(id: string | undefined): Region | undefined {
  return id ? BY_ID.get(id) : undefined;
}

export function regionsInView(west: number, south: number, east: number, north: number): Region[] {
  return REGIONS.filter(({bounds: [[w, s], [e, n]]}) => w <= east && e >= west && s <= north && n >= south);
}

export const REGION_BUSES_MAX_KM2 = 2000;

// center in view, not zoomed out
export function regionsShowingAllBuses(lng: number, lat: number, km2: number): Region[] {
  if (km2 >= REGION_BUSES_MAX_KM2) return [];
  return REGIONS.filter(
    ({allBuses, bounds: [[w, s], [e, n]]}) => allBuses && w <= lng && lng <= e && s <= lat && lat <= n
  );
}
