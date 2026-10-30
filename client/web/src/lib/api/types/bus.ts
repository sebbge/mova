import type {StopRef} from './stop';

export interface Point {
  lat: number;
  lng: number;
}

export type BusFreshness = 'live' | 'low_precision' | 'imprecise';

export interface Bus {
  id: string;
  unit: string;
  line: string;
  company: string;
  location: Point;
  speed?: number;
  origin?: string;
  destination?: string;
  headsign?: string;
  shape_id?: string;
  timestamp?: string;
  access?: string;
  emissions?: string;
  thermal_comfort?: string;
  delay_min?: number;
  bearing_deg?: number;
  stop?: StopRef;
  next_stop?: StopRef;
  freshness: BusFreshness;
  still: boolean;
  past_stop: boolean;
  photo_url?: string;
  logo_url?: string;
}

export interface RegionBus extends Bus {
  provider: string;
}
