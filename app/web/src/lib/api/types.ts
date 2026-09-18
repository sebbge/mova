// must match the server entities (app/server/src/bondi/entities)

import type {LineString} from 'geojson';

export interface Point {
  lat: number;
  lng: number;
}

export type BusFreshness = 'live' | 'low_precision' | 'imprecise';

export interface Bus {
  id: string;
  line: string;
  company: string;
  location: Point;
  speed: number;
  origin?: string;
  destination?: string;
  headsign?: string;
  timestamp?: string;
  access?: string;
  emissions?: string;
  thermal_comfort?: string;
  bearing_deg?: number;
  freshness: BusFreshness;
  still: boolean;
  past_stop: boolean;
  stale_count: number;
  miss_count: number;
  last_seen_epoch: number;
}

export interface NearbyStop {
  stop_id: string;
  stop_name: string;
  lat: number;
  lng: number;
  distance_meters: number;
  bearing_rad?: number;
}

export interface StopArrival {
  eta?: number;
  live: boolean;
  bus?: Bus;
}

export interface StopLine {
  line: string;
  destination?: string;
  company: string;
  arrivals: StopArrival[];
}

export interface StopDetail {
  lines: StopLine[];
}

export interface BusRoute {
  route_id: string;
  route_short_name: string;
  route_long_name?: string;
  agency_id?: string;
}

export interface RouteStop {
  stop_id: string;
  stop_name: string;
  stop_sequence: number;
  lat: number;
  lng: number;
  bearing_rad?: number;
}

export interface DirectionInfo {
  direction_id: number;
  headsign?: string;
}

export interface RouteShape {
  route_id: string;
  direction_id: number;
  headsign?: string;
  shape: LineString;
  stops: RouteStop[];
  available_directions: DirectionInfo[];
}
