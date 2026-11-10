import {api} from '~/lib/api/client';
import {REGIONS, type Region} from '~/lib/domain/regions';
import {lineSlug} from '~/lib/format';

export function regionPaths() {
  return REGIONS.map((region) => ({params: {region: region.slug}, props: {region}}));
}

// one point per shelter in the list, its siblings from the stop
export async function stopRefs(): Promise<{region: Region; id: string}[]> {
  const byRegion = await Promise.all(
    REGIONS.map((r) =>
      api
        .stops(r.id)
        .then((stops) =>
          Promise.all(
            stops.map(async (s) => ((s.providers?.length ?? 0) > 1 ? (await api.stop(s.id)).siblings : [s.id]))
          )
        )
    )
  );
  const seen = new Set<string>();
  return REGIONS.flatMap((region, i) =>
    byRegion[i].flat().flatMap((id) => (seen.has(id) ? [] : (seen.add(id), [{region, id}])))
  );
}

export async function stopPaths() {
  return (await stopRefs()).map(({region, id}) => ({params: {region: region.slug, id}, props: {region}}));
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
