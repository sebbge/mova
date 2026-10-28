import type {Map as MapLibreMap} from 'maplibre-gl';
import {createEffect, createSignal, onCleanup, Show} from 'solid-js';
import type {Bus} from '~/lib/api/types';
import {companyOutlineVar, companyPinVar} from '~/lib/ui/palette';
import styles from './BusGuide.module.css';
import {sheetOffset, uncovered} from './viewport';

const INSET = 28; // px
const GO_MS = 600;

// edge arrow for an out-of-view bus
export default function BusGuide(props: {map: MapLibreMap; bus: Bus | undefined}) {
  const [at, setAt] = createSignal<{x: number; y: number; angle: number} | null>(null);

  function place() {
    const bus = props.bus;
    if (!bus) {
      setAt(null);
      return;
    }
    const r = uncovered(props.map);
    const p = props.map.project([bus.location.lng, bus.location.lat]);
    if (p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom) {
      setAt(null);
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
    setAt({
      x: Math.min(r.right - INSET, Math.max(r.left + INSET, cx + dx * t)),
      y: Math.min(r.bottom - INSET, Math.max(r.top + INSET, cy + dy * t)),
      angle: (Math.atan2(dx, -dy) * 180) / Math.PI,
    });
  }

  createEffect(() => {
    const map = props.map;
    place();
    map.on('move', place);
    onCleanup(() => map.off('move', place));
  });

  function go(bus: Bus) {
    const map = props.map;
    map.easeTo({center: [bus.location.lng, bus.location.lat], offset: sheetOffset(map), duration: GO_MS});
  }

  return (
    <Show when={props.bus && at()}>
      {(a) => (
        <button
          class={styles.guide}
          style={{
            left: `${a().x}px`,
            top: `${a().y}px`,
            rotate: `${a().angle}deg`,
            '--pin': companyPinVar(props.bus?.company),
            '--pin-outline': companyOutlineVar(props.bus?.company),
          }}
          onClick={() => props.bus && go(props.bus)}
          aria-label='Ir al bus, fuera de la vista'>
          <svg viewBox='0 0 12 12' width='28' height='28' aria-hidden='true'>
            <path d='M6 0 L11 9 L6 7 L1 9 Z' />
          </svg>
        </button>
      )}
    </Show>
  );
}
