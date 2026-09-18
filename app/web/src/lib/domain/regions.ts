// id ISO 3166-2, slug subdivision only
export interface Region {
  id: string;
  slug: string;
  name: string;
  center: {lat: number; lng: number};
  zoom: number;
}

export const REGIONS: readonly Region[] = [
  {
    id: 'uy-mo',
    slug: 'mo',
    name: 'Montevideo',
    center: {lat: -34.9011, lng: -56.1645},
    zoom: 13,
  },
];

export const DEFAULT_REGION = REGIONS[0];
