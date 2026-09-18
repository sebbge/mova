import {REGIONS} from '~/lib/domain/regions';

export function regionPaths() {
  return REGIONS.map((region) => ({params: {region: region.slug}, props: {region}}));
}
