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
import type {Bus, RouteShape, StopItem} from '~/lib/api/types';
import {DEFAULT_REGION, regionsInView, type MapCamera} from '~/lib/domain/regions';
import {BusLayer} from './buses/BusLayer';
import {routeLayer} from './layers/route';
import {stopsLayer} from './layers/stops';
import styles from './MapCanvas.module.css';
import {RegionStops} from './RegionStops';
import {ATTRIBUTION, basemapStyle} from './style';
import {overlaps, sheetOffset, STOPS_MAX_KM2, stopsZoom, viewportKm2} from './viewport';

const CAMERA_MS = 400;
const FOCUS_MS = 500;
const FOLLOW_MS = 600;

export default function MapCanvas(props: {
  camera: MapCamera;
  shape: RouteShape | null; // one direction
  routeCompany?: string;
  buses: Bus[];
  selectedStopId: string | null;
  selectedSiblings: readonly string[];
  selectedStop?: {stop_id: string; lat: number; lng: number} | null; // camera before its region loads
  followBusId: string | null;
  onSelectStop?: (stopId: string, stopName?: string) => void;
  onSelectBus?: (bus: Bus) => void;
  onBackground?: () => void;
  onView?: (center: {lng: number; lat: number}, km2: number) => void;
}) {
  let container!: HTMLDivElement;
  const [map, setMap] = createSignal<MapLibreMap>();
  const [error, setError] = createSignal<string | null>(null);
  const [viewKm2, setViewKm2] = createSignal(Infinity);
  const tooWide = () => viewKm2() > STOPS_MAX_KM2;
  const stops = new RegionStops();

  onMount(() => {
    void stops.load([DEFAULT_REGION]);
    setWorkerUrl(workerUrl);
    addProtocol('pmtiles', new Protocol().tile);
    const camera = props.camera;
    const darkQuery = matchMedia('(prefers-color-scheme: dark)');

    let instance: MapLibreMap;
    try {
      instance = new MapLibreMap({
        container,
        style: basemapStyle(darkQuery.matches),
        center: [camera.center.lng, camera.center.lat],
        zoom: camera.zoom,
        minZoom: camera.minZoom,
        maxBounds: camera.bounds,
        fadeDuration: 0,
        attributionControl: false,
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
      if (instance.getSource('stops')) return;
      routeLayer.addSource(instance);
      stopsLayer.addImages(instance);
      stopsLayer.add(instance, 'stops');
      routeLayer.addLayers(instance, props.routeCompany);
      stopsLayer.add(instance, 'route-stops');
    };

    /* eslint-disable solid/reactivity -- maplibre event handlers read props when they fire */
    const onView = () => {
      const bounds = instance.getBounds();
      setViewKm2(viewportKm2(bounds));
      props.onView?.(instance.getCenter(), viewKm2());
      void stops.load(regionsInView(bounds.getWest(), bounds.getSouth(), bounds.getEast(), bounds.getNorth()));
    };

    instance.addControl(new NavigationControl({showCompass: false}), 'bottom-right');
    instance.addControl(new GeolocateControl({trackUserLocation: true, showAccuracyCircle: true}), 'bottom-right');

    instance.on('load', () => {
      addLayers();
      onView();
      setMap(instance);
    });
    instance.on('moveend', onView);

    for (const layer of stopsLayer.HITS) {
      instance.on('click', layer, (e) => {
        const p = e.features?.[0]?.properties;
        if (p?.stop_id) props.onSelectStop?.(String(p.stop_id), p.stop_name && String(p.stop_name));
      });
      instance.on('mouseenter', layer, () => (instance.getCanvas().style.cursor = 'pointer'));
      instance.on('mouseleave', layer, () => (instance.getCanvas().style.cursor = ''));
    }
    instance.on('click', (e) => {
      if (instance.queryRenderedFeatures(e.point, {layers: stopsLayer.HITS}).length === 0) props.onBackground?.();
    });

    // setStyle drops custom layers
    const onScheme = (e: MediaQueryListEvent) => {
      instance.setStyle(basemapStyle(e.matches));
      instance.once('idle', () => {
        addLayers();
        setMap(undefined);
        setMap(instance);
      });
    };
    darkQuery.addEventListener('change', onScheme);

    const buses = new BusLayer(instance, (bus) => props.onSelectBus?.(bus));
    /* eslint-enable solid/reactivity */
    createEffect(() => map() && buses.render(props.buses, props.followBusId, props.shape));

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
    if (m) routeLayer.paint(m, props.routeCompany);
  });

  createEffect(() => {
    const m = map();
    if (!m) return;
    const shape = props.shape;
    const onRoute = new Set(shape?.stops.map((s) => s.cluster) ?? []);
    const route: StopItem[] = shape
      ? shape.stops.map((s) => ({id: s.stop_id, name: s.stop_name, lat: s.lat, lng: s.lng}))
      : [];
    const rest = tooWide() ? [] : stops.all.filter((s) => !onRoute.has(s.id));
    const marks = {selectedId: props.selectedStopId, siblings: props.selectedSiblings};

    stopsLayer.set(m, 'route-stops', route, {...marks, dim: false});
    stopsLayer.set(m, 'stops', rest, {...marks, dim: onRoute.size > 0});
  });

  // pans in without zooming out
  createEffect(() => {
    const m = map();
    if (!m || !props.shape || props.followBusId) return;
    const bounds = new LngLatBounds();
    const points = props.shape.shape?.coordinates ?? props.shape.stops.map((p) => [p.lng, p.lat]);
    for (const [lng, lat] of points) bounds.extend([lng, lat]);
    if (!overlaps(m.getBounds(), bounds)) m.panTo(bounds.getCenter(), {duration: CAMERA_MS});
  });

  createEffect(() => {
    const m = map();
    const id = props.selectedStopId;
    if (!m || !id) return;
    const known = props.selectedStop?.stop_id === id ? props.selectedStop : undefined;
    const stop = stops.find(id) ?? props.shape?.stops.find((s) => s.stop_id === id) ?? known;
    if (stop) m.easeTo({center: [stop.lng, stop.lat], zoom: stopsZoom(m), offset: sheetOffset(m), duration: FOCUS_MS});
  });

  createEffect(() => {
    const m = map();
    if (!m || !props.followBusId) return;
    const bus = props.buses.find((b) => b.id === props.followBusId);
    if (bus) m.easeTo({center: [bus.location.lng, bus.location.lat], offset: sheetOffset(m), duration: FOLLOW_MS});
  });

  return (
    <>
      <div class={styles.map} ref={container} />
      <Switch>
        <Match when={error()}>
          <p class={`${styles.banner} ${styles.error}`}>{error()}</p>
        </Match>
        <Match when={tooWide() && !props.shape}>
          <p class={styles.banner}>Acercá el mapa para ver las paradas</p>
        </Match>
      </Switch>
      {/* eslint-disable-next-line solid/no-innerhtml */}
      <p class={styles.credit} innerHTML={ATTRIBUTION} />
    </>
  );
}
