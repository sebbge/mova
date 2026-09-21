import {PUBLIC_API_URL} from 'astro:env/client';
import type {Bus, BusRoute, NearbyStop, RouteShape, Stop, StopDetail, StopSchedule, StopSummary} from './types';

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

interface Options {
  signal?: AbortSignal;
}

async function request<T>(path: string, opts: Options = {}): Promise<T> {
  const res = await fetch(`${PUBLIC_API_URL}${path}`, {signal: opts.signal});

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(res.status, body?.error ?? res.statusText);
  }

  return res.json() as Promise<T>;
}

function query(params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) search.set(key, String(value));
  }
  const s = search.toString();
  return s ? `?${s}` : '';
}

export const api = {
  stops: (o?: Options) => request<StopSummary[]>('/stops', o),

  nearbyStops: (lat: number, lng: number, radius?: number, o?: Options) =>
    request<NearbyStop[]>(`/stops/nearby${query({lat, lng, radius})}`, o),

  stop: (stopId: string, o?: Options) => request<Stop>(`/stops/${stopId}`, o),

  stopArrivals: (stopId: string, o?: Options) => request<StopDetail>(`/stops/${stopId}/arrivals`, o),

  stopSchedule: (stopId: string, o?: Options) => request<StopSchedule>(`/stops/${stopId}/schedule`, o),

  routes: (o?: Options) => request<BusRoute[]>('/routes', o),

  searchRoutes: (q: string, o?: Options) => request<BusRoute[]>(`/routes/search${query({q})}`, o),

  routeShape: (routeId: string, directionId?: number, o?: Options) =>
    request<RouteShape>(`/routes/${routeId}/shape${query({direction_id: directionId})}`, o),

  lineBuses: (lineId: string, o?: Options) => request<Bus[]>(`/lines/${lineId}`, o),
};
