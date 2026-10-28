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
import {createEffect, createMemo, createSignal, Match, on, onCleanup, onMount, Show, Switch} from 'solid-js';
import type {FeatureCollection} from 'geojson';
import type {Bus, RouteShape} from '~/lib/api/types';
import {DEFAULT_REGION, regionsInView, type MapCamera} from '~/lib/domain/regions';
import {companyOutlineVar, companyPinVar} from '~/lib/ui/palette';
import {theme} from '~/lib/ui/theme';
import {BusLayer} from './buses/BusLayer';
import {routeLayer, type RouteView} from './layers/route';
import {stopsLayer} from './layers/stops';
import {terminalsLayer} from './layers/terminals';
import {tripLayer} from './layers/trip';
import styles from './MapCanvas.module.css';
import {RegionStops} from './RegionStops';
import {RouteTrack} from './RouteTrack';
import {ATTRIBUTION, basemapStyle} from './style';
import {fitCountry, overlaps, sheetOffset, STOPS_MAX_KM2, stopsZoom, uncovered, viewportKm2} from './viewport';

const CAMERA_MS = 400;
const FOCUS_MS = 500;
const FOLLOW_MS = 600;
const FAR_ZOOM = 13; // under it buses shrink
const GUIDE_INSET = 28; // px

export default function MapCanvas(props: {
  camera: MapCamera;
  shape: RouteShape | null; // one direction
  routeCompany?: string;
  buses: Bus[];
  selectedStopId: string | null;
  selectedSiblings: readonly string[];
  selectedStop?: {stop_id: string; lat: number; lng: number} | null; // camera before its region loads
  selectedBusId: string | null; // highlighted, camera stays
  followBusId: string | null; // user set, tapping the open bus's pin
  onSelectStop?: (stopId: string, stopName?: string) => void;
  onSelectBus?: (bus: Bus) => void;
  onBackground?: () => void;
  onView?: (center: {lng: number; lat: number}, km2: number) => void;
  trip?: FeatureCollection | null;
  tripFitKey?: string | null; // a new key frames the trip once
  picking?: boolean;
  onPick?: (point: {lat: number; lng: number}) => void;
}) {
  let container!: HTMLDivElement;
  const [map, setMap] = createSignal<MapLibreMap>();
  const [error, setError] = createSignal<string | null>(null);
  const [viewKm2, setViewKm2] = createSignal(Infinity);
  const [zoomStep, setZoomStep] = createSignal(0); // half steps
  const tooWide = () => viewKm2() > STOPS_MAX_KM2;
  const stops = new RegionStops();
  const track = createMemo(() => (props.shape?.shape ? new RouteTrack(props.shape.shape, props.shape.stops) : null));
  const isSelectedStop = (id: string) => id === props.selectedStopId || props.selectedSiblings.includes(id);
  const routeView = (): RouteView => ({shape: props.shape, track: track(), isSelected: isSelectedStop});

  onMount(() => {
    void stops.load([DEFAULT_REGION]);
    setWorkerUrl(workerUrl);
    addProtocol('pmtiles', new Protocol().tile);
    const camera = props.camera;
    let styledDark = theme.dark;

    let instance: MapLibreMap;
    try {
      instance = new MapLibreMap({
        container,
        style: basemapStyle(styledDark),
        center: [camera.center.lng, camera.center.lat],
        zoom: camera.zoom,
        fadeDuration: 0,
        attributionControl: false,
      });
    } catch (e) {
      console.error('[mova/map]', e); // no webgl
      setError('Este navegador no puede mostrar el mapa.');
      return;
    }

    // a 0x0 container leaves a blank canvas
    const resizer = new ResizeObserver(() => {
      instance.resize();
      fitCountry(instance, camera.bounds);
    });
    resizer.observe(container);
    fitCountry(instance, camera.bounds);

    instance.on('error', (e) => {
      setError('No pudimos cargar parte del mapa. Revisá tu conexión.');
      console.error('[mova/map]', e.error ?? e);
    });

    const addLayers = () => {
      stopsLayer.bakeImages(instance);
      if (instance.getSource('stops')) return;
      terminalsLayer.addSource(instance);
      routeLayer.addSources(instance);
      terminalsLayer.addLayers(instance);
      stopsLayer.add(instance);
      routeLayer.addLayers(instance, props.routeCompany);
      tripLayer.add(instance);
    };

    /* eslint-disable solid/reactivity -- maplibre event handlers read props when they fire */
    const onView = () => {
      const bounds = instance.getBounds();
      setViewKm2(viewportKm2(bounds));
      props.onView?.(instance.getCenter(), viewKm2());
      void stops.load(regionsInView(bounds.getWest(), bounds.getSouth(), bounds.getEast(), bounds.getNorth()));
    };

    // top, the sheet covers the bottom
    instance.addControl(new NavigationControl(), 'top-right');
    instance.addControl(new GeolocateControl({trackUserLocation: true, showAccuracyCircle: true}), 'top-right');

    const stepZoom = () => setZoomStep(Math.floor(instance.getZoom() * 2) / 2);
    instance.on('zoom', stepZoom);

    instance.on('load', () => {
      stepZoom();
      addLayers();
      onView();
      setMap(instance);
    });
    instance.on('moveend', onView);

    const stopHits = [...routeLayer.HITS, ...stopsLayer.HITS];
    const hits = [...stopHits, terminalsLayer.HIT];
    for (const layer of hits) {
      instance.on('click', layer, (e) => {
        // stop inside the terminal takes the tap
        if (layer === terminalsLayer.HIT && instance.queryRenderedFeatures(e.point, {layers: stopHits}).length) {
          return;
        }
        const p = e.features?.[0]?.properties;
        if (p?.stop_id) props.onSelectStop?.(String(p.stop_id), p.stop_name && String(p.stop_name));
      });
      instance.on('mouseenter', layer, () => (instance.getCanvas().style.cursor = 'pointer'));
      instance.on('mouseleave', layer, () => (instance.getCanvas().style.cursor = ''));
    }
    instance.on('click', (e) => {
      if (instance.queryRenderedFeatures(e.point, {layers: hits}).length > 0) return;
      if (props.picking) props.onPick?.({lat: e.lngLat.lat, lng: e.lngLat.lng});
      else props.onBackground?.();
    });

    // setStyle drops custom layers
    createEffect(
      on(
        () => theme.dark,
        (dark) => {
          if (dark === styledDark) return;
          styledDark = dark;
          instance.setStyle(basemapStyle(dark));
          instance.once('idle', () => {
            addLayers();
            setMap(undefined);
            setMap(instance);
          });
        },
        {defer: true}
      )
    );

    const buses = new BusLayer(instance, (bus) => props.onSelectBus?.(bus));
    instance.on('move', () => updateGuide(instance));
    /* eslint-enable solid/reactivity */
    createEffect(() => map() && buses.render(props.buses, props.selectedBusId, props.shape, zoomStep() < FAR_ZOOM));
    createEffect(() => map() && updateGuide(instance));

    onCleanup(() => {
      buses.destroy();
      resizer.disconnect();
      instance.remove();
    });
  });

  createEffect(() => {
    const m = map();
    if (m) routeLayer.set(m, routeView());
  });

  createEffect(() => {
    const m = map();
    if (m) routeLayer.cut(m, routeView(), zoomStep());
  });

  createEffect(() => {
    const m = map();
    if (m) routeLayer.paint(m, props.routeCompany);
  });

  createEffect(() => {
    const m = map();
    if (m) tripLayer.set(m, props.trip ?? null);
  });

  createEffect(() => {
    const m = map();
    if (m) m.getCanvas().style.cursor = props.picking ? 'crosshair' : '';
  });

  let fittedTrip: string | null = null;
  createEffect(() => {
    const m = map();
    const key = props.tripFitKey;
    if (!m || !props.trip || !key || key === fittedTrip) return;
    fittedTrip = key;
    tripLayer.fit(m, props.trip);
  });

  createEffect(() => {
    const m = map();
    if (!m) return;
    const onRoute = new Set(props.shape?.stops.map((s) => s.cluster) ?? []);
    const rest = tooWide() ? [] : stops.all.filter((s) => !onRoute.has(s.id));
    const marks = {selectedId: props.selectedStopId, siblings: props.selectedSiblings};

    stopsLayer.set(m, rest, {...marks, dim: onRoute.size > 0});
    terminalsLayer.set(m, tooWide() ? [] : stops.all, {...marks, dim: onRoute.size > 0});
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

  // edge arrow for an out-of-view bus
  const [guide, setGuide] = createSignal<{x: number; y: number; angle: number; bus: Bus} | null>(null);

  function updateGuide(m: MapLibreMap) {
    const bus = props.selectedBusId ? props.buses.find((b) => b.id === props.selectedBusId) : undefined;
    if (!bus) {
      setGuide(null);
      return;
    }
    const r = uncovered(m);
    const p = m.project([bus.location.lng, bus.location.lat]);
    if (p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom) {
      setGuide(null);
      return;
    }
    const cx = (r.left + r.right) / 2;
    const cy = (r.top + r.bottom) / 2;
    const dx = p.x - cx;
    const dy = p.y - cy;
    const t = Math.min(
      (r.right - r.left) / 2 / Math.max(Math.abs(dx), 1e-6),
      (r.bottom - r.top) / 2 / Math.max(Math.abs(dy), 1e-6)
    );
    setGuide({
      x: Math.min(r.right - GUIDE_INSET, Math.max(r.left + GUIDE_INSET, cx + dx * t)),
      y: Math.min(r.bottom - GUIDE_INSET, Math.max(r.top + GUIDE_INSET, cy + dy * t)),
      angle: (Math.atan2(dx, -dy) * 180) / Math.PI,
      bus,
    });
  }

  function goToGuided(bus: Bus) {
    const m = map();
    if (m) m.easeTo({center: [bus.location.lng, bus.location.lat], offset: sheetOffset(m), duration: FOLLOW_MS});
  }

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
      <Show when={guide()}>
        {(g) => (
          <button
            class={styles.guide}
            style={{
              left: `${g().x}px`,
              top: `${g().y}px`,
              rotate: `${g().angle}deg`,
              '--pin': companyPinVar(g().bus.company),
              '--pin-outline': companyOutlineVar(g().bus.company),
            }}
            onClick={() => goToGuided(g().bus)}
            aria-label='Ir al bus, fuera de la vista'>
            <svg viewBox='0 0 12 12' width='28' height='28' aria-hidden='true'>
              <path d='M6 0 L11 9 L6 7 L1 9 Z' />
            </svg>
          </button>
        )}
      </Show>
      {/* eslint-disable-next-line solid/no-innerhtml */}
      <p class={styles.credit} innerHTML={ATTRIBUTION} />
    </>
  );
}
