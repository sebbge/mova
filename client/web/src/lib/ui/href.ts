import {regionById} from '~/lib/domain/regions';
import {lineSlug} from '~/lib/format';

export const href = {
  region: (region: string) => `/${region}`,
  map: (params?: Record<string, string>) => `/mapa${params ? `?${new URLSearchParams(params)}` : ''}`,
  stop: (region: string, id: string) => `/${region}/parada/${id}`,
  line: (region: string, line: string, variant?: {route: string; direction: number}) =>
    `/${region}/linea/${lineSlug(line)}${variant ? `?v=${variant.route}&d=${variant.direction}` : ''}`,
  // no region -> map
  lineOrMap: (area: string | undefined, line: string) => {
    const region = regionById(area);
    return region ? href.line(region.slug, line) : href.map({linea: line});
  },
};
