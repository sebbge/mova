import {PUBLIC_API_URL} from 'astro:env/client';
import type {BusRoute, NearbyStop, RouteShape, StopDetail} from './types';

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
  nearbyStops: (lat: number, lng: number, radius?: number, o?: Options) =>
    request<NearbyStop[]>(`/stops/nearby${query({lat, lng, radius})}`, o),

  stopDetail: (stopId: string, o?: Options) => request<StopDetail>(`/stops/${stopId}`, o),

  searchRoutes: (q: string, o?: Options) => request<BusRoute[]>(`/routes/search${query({q})}`, o),

  routeShape: (routeId: string, directionId = 0, o?: Options) =>
    request<RouteShape>(`/routes/${routeId}/shape${query({direction_id: directionId})}`, o),
};
