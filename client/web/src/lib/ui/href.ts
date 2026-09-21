import {lineSlug} from '~/lib/format';

export const href = {
  region: (region: string) => `/${region}`,
  map: (region: string, params?: Record<string, string>) =>
    `/${region}/mapa${params ? `?${new URLSearchParams(params)}` : ''}`,
  stop: (region: string, id: string) => `/${region}/parada/${id}`,
  line: (region: string, line: string, variant?: {route: string; direction: number}) =>
    `/${region}/linea/${lineSlug(line)}${variant ? `?v=${variant.route}&d=${variant.direction}` : ''}`,
};
