import {PUBLIC_API_URL} from 'astro:env/client';

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export interface Options {
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

export async function request<T>(path: string, opts: Options = {}): Promise<T> {
  return (await send(path, opts)).json() as Promise<T>;
}

const memo = new Map<string, {until: number; value: Promise<unknown>}>();

// gtfs data holds until the next sync, and max-age says how long
export function gtfs<T>(path: string, opts: Options = {}): Promise<T> {
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
