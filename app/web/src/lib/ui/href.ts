export const href = {
  region: (region: string) => `/${region}`,
  map: (region: string) => `/${region}/mapa`,
  stop: (region: string, id: string) => `/${region}/parada?id=${encodeURIComponent(id)}`,
  line: (region: string, line: string, variant?: {route: string; direction: number}) => {
    const params = new URLSearchParams({l: line});
    if (variant) {
      params.set('v', variant.route);
      params.set('d', String(variant.direction));
    }
    return `/${region}/linea?${params}`;
  },
};
