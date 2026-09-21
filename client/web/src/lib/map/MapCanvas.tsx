import {
  addProtocol,
  GeolocateControl,
  LngLatBounds,
  Map as MapLibreMap,
  NavigationControl,
  setWorkerUrl,
} from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';
import {Protocol} from 'pmtiles';
import {createEffect, createSignal, Match, onCleanup, onMount, Switch} from 'solid-js';
import type {Bus, RouteShape} from '~/lib/api/types';
import type {Region} from '~/lib/domain/regions';
import {BusLayer} from './buses/BusLayer';
import {routeLayer} from './layers/route';
import {stopsLayer, type StopPoint} from './layers/stops';
import styles from './MapCanvas.module.css';
import {NearbyStops} from './NearbyStops';
import {basemapStyle} from './style';

const CAMERA_MS = 400;
const FOLLOW_MS = 600;

export default function MapCanvas(props: {
  region: Region;
  shape: RouteShape | null; // one direction
  buses: Bus[];
  selectedStopId: string | null;
  followBusId: string | null;
  onSelectStop?: (stopId: string) => void;
  onSelectBus?: (bus: Bus) => void;
  onBackground?: () => void;
}) {
  let container!: HTMLDivElement;
  const [map, setMap] = createSignal<MapLibreMap>();
  const [error, setError] = createSignal<string | null>(null);
  const nearby = new NearbyStops();

  onMount(() => {
    const region = props.region;
    setWorkerUrl(workerUrl);
    addProtocol('pmtiles', new Protocol().tile);
    const darkQuery = matchMedia('(prefers-color-scheme: dark)');

    let instance: MapLibreMap;
    try {
      instance = new MapLibreMap({
        container,
        style: basemapStyle(darkQuery.matches, region.id),
        center: [region.center.lng, region.center.lat],
        zoom: region.zoom,
        minZoom: region.minZoom,
        maxBounds: region.bounds,
        attributionControl: {compact: true},
      });
    } catch (e) {
      setError((e as Error).message); // no webgl
      return;
    }

    const resizer = new ResizeObserver(() => instance.resize()); // a 0x0 container leaves a blank canvas
    resizer.observe(container);

    instance.on('error', (e) => {
      setError(e.error?.message ?? 'No se pudo cargar el mapa');
      console.error('[bondi/map]', e.error ?? e);
    });

    const addLayers = () => {
      routeLayer.add(instance);
      stopsLayer.add(instance);
    };

    instance.addControl(new NavigationControl({showCompass: false}), 'bottom-right');
    instance.addControl(new GeolocateControl({trackUserLocation: true, showAccuracyCircle: true}), 'bottom-right');

    /* eslint-disable solid/reactivity -- maplibre event handlers read props when they fire */
    instance.on('load', () => {
      addLayers();
      setMap(instance);
      if (!props.shape) void nearby.refresh(instance);
    });
    instance.on('moveend', () => {
      if (!props.shape) nearby.refreshSoon(instance);
    });

    instance.on('click', stopsLayer.HIT, (e) => {
      const stopId = e.features?.[0]?.properties?.stop_id;
      if (stopId) props.onSelectStop?.(String(stopId));
    });
    instance.on('click', (e) => {
      if (instance.queryRenderedFeatures(e.point, {layers: [stopsLayer.HIT]}).length === 0) props.onBackground?.();
    });
    for (const layer of stopsLayer.LAYERS) {
      instance.on('mouseenter', layer, () => (instance.getCanvas().style.cursor = 'pointer'));
      instance.on('mouseleave', layer, () => (instance.getCanvas().style.cursor = ''));
    }

    // setStyle drops custom layers
    const onScheme = (e: MediaQueryListEvent) => {
      instance.setStyle(basemapStyle(e.matches, region.id));
      instance.once('idle', () => {
        addLayers();
        setMap(undefined);
        setMap(instance);
      });
    };
    darkQuery.addEventListener('change', onScheme);

    const buses = new BusLayer(instance, (bus) => props.onSelectBus?.(bus));
    /* eslint-enable solid/reactivity */
    createEffect(() => map() && buses.render(props.buses));

    onCleanup(() => {
      buses.destroy();
      resizer.disconnect();
      darkQuery.removeEventListener('change', onScheme);
      instance.remove();
    });
  });

  createEffect(() => {
    const m = map();
    if (m) routeLayer.set(m, props.shape);
  });

  createEffect(() => {
    const m = map();
    if (!m) return;
    const shape = props.shape;
    const onRoute = new Set(shape?.stops.map((s) => s.stop_id) ?? []);
    const visible: StopPoint[] = shape ? [...shape.stops, ...nearby.stops] : nearby.stops;
    const seen = new Set<string>();
    const unique = visible.filter((s) => !seen.has(s.stop_id) && seen.add(s.stop_id));
    stopsLayer.set(m, unique, onRoute, props.selectedStopId);
  });

  // pans in without zooming out
  createEffect(() => {
    const m = map();
    if (!m || !props.shape) return;
    const bounds = new LngLatBounds();
    for (const [lng, lat] of props.shape.shape.coordinates) bounds.extend([lng, lat]);
    if (!overlaps(m.getBounds(), bounds)) m.panTo(bounds.getCenter(), {duration: CAMERA_MS});
  });

  createEffect(() => {
    const m = map();
    if (!m || !props.followBusId) return;
    const bus = props.buses.find((b) => b.id === props.followBusId);
    if (bus) m.easeTo({center: [bus.location.lng, bus.location.lat], duration: FOLLOW_MS});
  });

  return (
    <>
      <div class={styles.map} ref={container} />
      <Switch>
        <Match when={error()}>
          <p class={`${styles.banner} ${styles.error}`}>{error()}</p>
        </Match>
        <Match when={nearby.zoomedOut && !props.shape}>
          <p class={styles.banner}>Acercá el mapa para ver las paradas</p>
        </Match>
      </Switch>
    </>
  );
}

function overlaps(a: LngLatBounds, b: LngLatBounds): boolean {
  return (
    a.getWest() <= b.getEast() &&
    a.getEast() >= b.getWest() &&
    a.getSouth() <= b.getNorth() &&
    a.getNorth() >= b.getSouth()
  );
}
