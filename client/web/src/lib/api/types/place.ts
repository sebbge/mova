export type PlaceKind = 'zone' | 'poi' | 'street' | 'corner';

export interface PlaceHit {
  id: string;
  kind: PlaceKind;
  name: string;
  category: string;
  context?: string;
  lat: number;
  lng: number;
  area?: string;
}
