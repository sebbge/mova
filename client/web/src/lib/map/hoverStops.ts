import type {Map as MapLibreMap, PointLike} from 'maplibre-gl';

export interface StopHit {
  id: string;
  cluster: string; // shelter
  name: string;
  service: string; // combo, e.g. urban+suburban
  at: {lng: number; lat: number};
}

// mouse only, hidden while moving and under buses
export function hoverStops(map: MapLibreMap, layers: readonly string[], onHover: (hit: StopHit | null) => void) {
  let hovered: string | null = null;
  let moving = false;

  const leave = () => {
    if (hovered === null) return;
    hovered = null;
    onHover(null);
  };

  const hover = (point: PointLike) => {
    if (moving) return;
    const present = layers.filter((l) => map.getLayer(l));
    const feature = present.length ? map.queryRenderedFeatures(point, {layers: present})[0] : undefined;
    const props = feature?.properties;
    if (!feature || !props?.stop_id) return leave();

    const id = String(props.stop_id);
    if (id === hovered) return;
    hovered = id;
    const [lng, lat] =
      feature.geometry.type === 'Point' ? feature.geometry.coordinates : map.unproject(point).toArray();
    onHover({
      id,
      cluster: String(props.cluster ?? id),
      name: String(props.stop_name ?? ''),
      service: String(props.service ?? ''),
      at: {lng, lat},
    });
  };

  map.on('mousemove', (e) => {
    const target = e.originalEvent.target;
    if (target instanceof Element && target.closest('.maplibregl-marker')) leave();
    else hover(e.point);
  });
  map.getCanvas().addEventListener('mouseleave', leave);
  map.on('movestart', () => {
    moving = true;
    leave();
  });
  map.on('moveend', () => (moving = false));
}
