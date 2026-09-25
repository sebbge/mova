import {api} from '~/lib/api/client';
import {REGIONS} from '~/lib/domain/regions';
import {lineSlug} from '~/lib/format';

export function regionPaths() {
  return REGIONS.map((region) => ({params: {region: region.slug}, props: {region}}));
}

export async function stopPaths() {
  const byRegion = await Promise.all(REGIONS.map((r) => api.stops(r.id)));
  return REGIONS.flatMap((region, i) =>
    byRegion[i].map((stop) => ({params: {region: region.slug, id: stop.id}, props: {region}}))
  );
}

// one department per line, no duplicate pages
export async function lineAreas(): Promise<{line: string; area?: string}[]> {
  const byLine = new Map<string, string | undefined>();
  for (const r of await api.routes()) if (!byLine.has(r.route_short_name)) byLine.set(r.route_short_name, r.area);
  return [...byLine].map(([line, area]) => ({line, area}));
}

export async function linePaths() {
  const lines = await lineAreas();
  return REGIONS.flatMap((region) =>
    lines
      .filter((l) => l.area === region.id)
      .map(({line}) => ({params: {region: region.slug, line: lineSlug(line)}, props: {region, line}}))
  );
}
