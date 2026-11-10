import type {LineString} from 'geojson';
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
  area?: string;
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

export interface RouteTimes {
  times: (number | null)[];
}

export type DayType = 'habil' | 'sabado' | 'domingo';

export interface ScheduleDeparture {
  secs: number;
  minute: number;
  zonal?: boolean;
  interpolated?: boolean;
  next?: boolean;
}

export interface ScheduleHour {
  hour: number;
  late: boolean;
  departures: ScheduleDeparture[];
}

export interface RouteSchedule {
  day: DayType;
  date: string;
  hours: ScheduleHour[];
}
