import {api} from '~/lib/api/client';
import {REGIONS} from '~/lib/domain/regions';
import {lineSlug} from '~/lib/format';

export function regionPaths() {
  return REGIONS.map((region) => ({params: {region: region.slug}, props: {region}}));
}

export async function stopPaths() {
  const stops = await api.stops();
  return REGIONS.flatMap((region) =>
    stops.map((stop) => ({params: {region: region.slug, id: stop.stop_id}, props: {region}}))
  );
}

export async function linePaths() {
  const lines = [...new Set((await api.routes()).map((r) => r.route_short_name))];
  return REGIONS.flatMap((region) =>
    lines.map((line) => ({params: {region: region.slug, line: lineSlug(line)}, props: {region, line}}))
  );
}
