import type {BusRoute, StopItem} from '~/lib/api/types';
import {providerOf, serviceOf} from '~/lib/domain/providers';
import {REGIONS} from '~/lib/domain/regions';

// inter lines reach every department
export type Coverage = 'urban' | 'suburban' | 'inter';

export interface Department {
  id: string;
  slug: string;
  name: string;
  coverage: Coverage;
  lines: number; // anchored here only
  stops: number;
  liveOnly: boolean; // Paysandú, Rivera
}

export interface CoverageSummary {
  departments: Department[];
  lines: number;
  stops: number;
}

export function coverage(routes: BusRoute[], stopsByRegion: StopItem[][]): CoverageSummary {
  // national lines have no area
  const anchored = new Map<string, BusRoute[]>();
  for (const r of routes) anchored.set(r.area ?? '', [...(anchored.get(r.area ?? '') ?? []), r]);
  const names = (rs: BusRoute[] = []) => new Set(rs.map((r) => r.route_short_name)).size;

  const departments = REGIONS.map((region, i): Department => {
    const own = anchored.get(region.id) ?? [];
    const stops = stopsByRegion[i];
    const urban = region.allBuses || own.some((r) => !['dnt', 'inter'].includes(providerOf(r.route_id)));
    const suburban = own.length > 0 || stops.some((s) => serviceOf(s.id) === 'suburban');
    const lines = names(own);
    return {
      id: region.id,
      slug: region.slug,
      name: region.name,
      coverage: urban ? 'urban' : suburban ? 'suburban' : 'inter',
      lines,
      stops: stops.length,
      liveOnly: Boolean(region.allBuses) && lines === 0,
    };
  });

  return {
    departments,
    lines: departments.reduce((n, d) => n + d.lines, names(anchored.get(''))),
    stops: departments.reduce((n, d) => n + d.stops, 0),
  };
}
