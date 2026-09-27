// must match the server entities (lib/server/src/domain/entities)

import type {LineString} from 'geojson';

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
}

export interface RegionBus extends Bus {
  provider: string;
}

export interface StopRef {
  id: string;
  name: string;
}

// another operator's bus, no eta
export type StopPosition = 'at' | 'next';

export interface StopArrival {
  zonal?: boolean; // zone marker
  interpolated?: boolean; // estimated between two points
  stop_position?: StopPosition;
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

export interface ScheduledLine {
  zonal?: boolean;
  interpolated?: boolean;
  line: string;
  destination: string | null;
  departures: number[]; // s since today's midnight
}

export interface StopSchedule {
  service_date: string;
  lines: ScheduledLine[];
}

export interface StopDetail {
  lines: StopLine[];
}

export interface Stop {
  stop_id: string;
  stop_name: string;
  area?: string; // ISO 3166-2
  station?: string; // platforms only
  platform?: string;
  lat: number;
  lng: number;
  bearing_rad?: number;
  siblings: string[]; // same point, leading first
  lines: StopRoute[];
}

export interface StopRoute {
  line: string;
  destinations: string[];
}

export interface StopItem {
  id: string;
  name: string;
  lat: number;
  lng: number;
  providers?: string[];
  station?: string; // platforms only
  platform?: string;
  footprint?: number[];
}

export interface BusRoute {
  route_id: string;
  route_short_name: string;
  route_long_name?: string;
  agency_id?: string;
  region: string;
  area?: string;
}

export interface RouteStop {
  stop_id: string;
  cluster: string; // shelter ref
  station?: string; // platforms only
  platform?: string;
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
  shape_id: string | null;
  direction_id: number;
  headsign?: string;
  shape: LineString | null;
  stops: RouteStop[];
  available_directions: DirectionInfo[];
}
