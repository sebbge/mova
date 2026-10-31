import type {Bus} from './bus';

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
  source?: 'schedule' | 'upcoming' | 'corridor' | 'projection' | 'fallback' | 'ghost' | 'position' | 'relayed';
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
  generated_at?: string;
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

export interface StopHit {
  id: string;
  name: string;
  lat: number;
  lng: number;
  area?: string;
  station?: string;
  platform?: string;
}
