import {PUBLIC_API_URL} from 'astro:env/client';
import type {Bus, BusRoute, RegionBus, RouteShape, Stop, StopDetail, StopItem, StopSchedule} from './types';

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

function apiUrl(): string {
  if (typeof window !== 'undefined' && PUBLIC_API_URL.includes('localhost')) {
    return PUBLIC_API_URL.replace('localhost', window.location.hostname);
  }
  return PUBLIC_API_URL;
}

async function send(path: string, opts: Options = {}): Promise<Response> {
  const res = await fetch(`${apiUrl()}${path}`, {signal: opts.signal});

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(res.status, body?.error ?? res.statusText);
  }

  return res;
}

async function request<T>(path: string, opts: Options = {}): Promise<T> {
  return (await send(path, opts)).json() as Promise<T>;
}

const memo = new Map<string, {until: number; value: Promise<unknown>}>();

// gtfs data holds until the next sync, and max-age says how long
function gtfs<T>(path: string, opts: Options = {}): Promise<T> {
  if (opts.signal) return request<T>(path, opts);

  const hit = memo.get(path);
  if (hit && hit.until > Date.now()) return hit.value as Promise<T>;

  const entry = {until: Infinity, value: Promise.resolve() as Promise<unknown>};
  entry.value = send(path).then((res) => {
    const age = Number(/max-age=(\d+)/.exec(res.headers.get('cache-control') ?? '')?.[1] ?? 0);
    entry.until = Date.now() + age * 1000;
    return res.json();
  });
  entry.value.catch(() => memo.delete(path));
  memo.set(path, entry);
  return entry.value as Promise<T>;
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

  stopSchedule: (stopRef: string, o?: Options) => gtfs<StopSchedule>(`/stop/schedule/${refPath(stopRef)}`, o),

  routes: (o?: Options) => gtfs<BusRoute[]>('/routes', o),

  searchRoutes: (q: string, o?: Options) => gtfs<BusRoute[]>(`/routes/search${query({q})}`, o),

  routeShape: (routeRef: string, directionId?: number, o?: Options) =>
    gtfs<RouteShape>(`/route/shape/${refPath(routeRef)}${query({direction_id: directionId})}`, o),

  shape: (shapeRef: string, o?: Options) => gtfs<RouteShape>(`/shape/${refPath(shapeRef)}`, o),

  lineBuses: (provider: string, line: string, o?: Options) =>
    request<Bus[]>(`/line/buses/${provider}/${encodeURIComponent(line)}`, o),

  regionBuses: (region: string, o?: Options) => request<RegionBus[]>(`/buses${query({region})}`, o),
};
