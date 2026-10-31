import type {
  Bus,
  BusRoute,
  Category,
  Itinerary,
  Payment,
  PlaceHit,
  DayType,
  RegionBus,
  RouteSchedule,
  RouteShape,
  RouteTimes,
  Stop,
  StopDetail,
  StopHit,
  StopItem,
  StopSchedule,
  TravelMode,
} from './types';
import {gtfs, request, type Options} from './http';

export {ApiError} from './http';

function near(at?: {lat: number; lng: number}, digits = 3): string | undefined {
  return at ? `${at.lat.toFixed(digits)},${at.lng.toFixed(digits)}` : undefined;
}

function query(params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) search.set(key, String(value));
  }
  const s = search.toString();
  return s ? `?${s}` : '';
}

// `stm-3336` -> `stm/3336`
function refPath(ref: string): string {
  const i = ref.indexOf('-');
  return `${ref.slice(0, i)}/${encodeURIComponent(ref.slice(i + 1))}`;
}

export const api = {
  stops: (area: string, o?: Options) => gtfs<StopItem[]>(`/stops?area=${encodeURIComponent(area)}`, o),

  stop: (stopRef: string, o?: Options) => gtfs<Stop>(`/stop/${refPath(stopRef)}`, o),

  stopArrivals: (stopRef: string, o?: Options) => request<StopDetail>(`/stop/arrivals/${refPath(stopRef)}`, o),

  // fleet memory only
  stopBuses: (stopRef: string, ids: string[], o?: Options) =>
    request<Bus[]>(`/stop/buses/${refPath(stopRef)}${query({ids: ids.join(',')})}`, o),

  stopSchedule: (stopRef: string, o?: Options) => gtfs<StopSchedule>(`/stop/schedule/${refPath(stopRef)}`, o),

  routes: (o?: Options) => gtfs<BusRoute[]>('/routes', o),

  searchRoutes: (q: string, o?: Options) => gtfs<BusRoute[]>(`/routes/search${query({q})}`, o),

  routeShape: (routeRef: string, directionId?: number, o?: Options) =>
    gtfs<RouteShape>(`/route/shape/${refPath(routeRef)}${query({direction_id: directionId})}`, o),

  shape: (shapeRef: string, o?: Options) => gtfs<RouteShape>(`/shape/${refPath(shapeRef)}`, o),

  // without 'day' the server marks the next departure
  routeSchedule: (routeRef: string, at: {direction: number; stop: string; day: DayType | null}, o?: Options) =>
    request<RouteSchedule>(
      `/route/schedule/${refPath(routeRef)}${query({direction_id: at.direction, stop: at.stop, day: at.day ?? undefined})}`,
      o
    ),

  // changes with the clock, outside the gtfs memo
  routeTimes: (routeRef: string, at: {direction: number; shapeId: string | null; stop: string | null}, o?: Options) =>
    request<RouteTimes>(
      `/route/times/${refPath(routeRef)}${query({direction_id: at.direction, shape_id: at.shapeId ?? undefined, stop: at.stop ?? undefined})}`,
      o
    ),

  lineBuses: (provider: string, line: string, o?: Options) =>
    request<Bus[]>(`/line/buses/${provider}/${encodeURIComponent(line)}`, o),

  regionBuses: (region: string, o?: Options) => request<RegionBus[]>(`/buses${query({region})}`, o),

  searchStops: (q: string, at?: {lat: number; lng: number}, o?: Options) =>
    gtfs<StopHit[]>(`/stops/search${query({q, near: near(at)})}`, o),

  searchPlaces: (q: string, at?: {lat: number; lng: number}, o?: Options) =>
    request<PlaceHit[]>(`/places/search${query({q, near: near(at)})}`, o),

  place: (id: string, o?: Options) => request<PlaceHit>(`/place/${encodeURIComponent(id)}`, o),

  reversePlace: (at: {lat: number; lng: number}, o?: Options) =>
    request<PlaceHit | null>(`/places/reverse${query({at: near(at, 5)})}`, o),

  plan: (
    p: {from: string; to: string; at?: string; payment?: Payment; category?: Category; mode?: TravelMode},
    o?: Options
  ) => request<Itinerary[]>(`/plan${query(p)}`, o),
};
