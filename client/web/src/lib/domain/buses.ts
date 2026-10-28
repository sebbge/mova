import type {Bus, RouteShape} from '~/lib/api/types';

// destination without a shape, else true
export function follows(bus: Bus, shape: RouteShape): boolean {
  if (bus.shape_id) return bus.shape_id === shape.shape_id;
  if (!bus.destination || !shape.headsign) return true;
  return bus.destination.toUpperCase() === shape.headsign.toUpperCase();
}
