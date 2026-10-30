import type {Bus, RouteShape} from '~/lib/api/types';

// same destination, else same shape, else true
export function follows(bus: Bus, shape: RouteShape): boolean {
  if (bus.shape_id && bus.shape_id === shape.shape_id) return true;
  if (bus.destination && shape.headsign) {
    return bus.destination.trim().toUpperCase() === shape.headsign.trim().toUpperCase();
  }
  return !bus.shape_id;
}
