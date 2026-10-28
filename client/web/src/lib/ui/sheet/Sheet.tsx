import {createSignal, onCleanup, onMount, Show, type JSX} from 'solid-js';
import {transition, fly} from '~/lib/ui/motion';
import {SheetDragContext, type SheetDrag} from './context';
import styles from './Sheet.module.css';

const SNAPS = [0.22, 0.48, 0.88]; // share of screen height: peek, half, full
const DRAG_THRESHOLD = 0.06; // share of screen height
const FLICK_PX = 12;
const FLICK_SPEED = 0.5; // px/ms

export default function Sheet(props: {
  open: boolean;
  onBack?: () => void; // only with a previous step
  onClose?: () => void;
  sticky?: boolean; // drag down and escape never close it
  children: JSX.Element;
}) {
  const [snap, setSnap] = createSignal(1);
  const [dragging, setDragging] = createSignal(false);
  const [dragOffset, setDragOffset] = createSignal(0);

  const clampSnap = (n: number) => Math.max(0, Math.min(n, SNAPS.length - 1));

  function start(e: PointerEvent) {
    setDragging(true);
    setDragOffset(0);
    const startY = e.clientY;
    const startAt = performance.now();
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);

    const move = (ev: PointerEvent) => setDragOffset(ev.clientY - startY);
    const release = () => {
      target.releasePointerCapture(e.pointerId);
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', release);

      const offset = dragOffset();
      const threshold = innerHeight * DRAG_THRESHOLD;
      const flick = Math.abs(offset) > FLICK_PX && Math.abs(offset) / (performance.now() - startAt) > FLICK_SPEED;
      const lower = offset > threshold || (flick && offset > 0);
      const raise = offset < -threshold || (flick && offset < 0);
      const next = snap() + (lower ? -1 : raise ? 1 : 0);
      if (next < 0 && !props.sticky) props.onClose?.();
      setSnap(clampSnap(next));
      setDragging(false);
      setDragOffset(0);
    };

    addEventListener('pointermove', move);
    addEventListener('pointerup', release);
  }

  const drag: SheetDrag = {
    start,
    key(e) {
      if (e.key === 'ArrowUp') setSnap(clampSnap(snap() + 1));
      else if (e.key === 'ArrowDown') setSnap(clampSnap(snap() - 1));
      else return;
      e.preventDefault();
    },
    get snap() {
      return snap();
    },
    max: SNAPS.length - 1,
  };

  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && e.target.closest('input, textarea, select, [contenteditable]');
      if (e.key === 'Escape' && !props.sticky && !typing) props.onClose?.();
    };
    addEventListener('keydown', onKeyDown);
    onCleanup(() => removeEventListener('keydown', onKeyDown));
  });

  return (
    <Show when={props.open}>
      <aside
        ref={(el) => transition(el, fly('100%', 1), {enter: 240, leave: 160})}
        class={styles.sheet}
        classList={{[styles.dragging]: dragging()}}
        style={{
          '--sheet-nav-start': props.onBack ? 'var(--tap)' : undefined,
          height: `${SNAPS[snap()] * 100}dvh`,
          transform: `translateY(${Math.max(0, dragOffset())}px)`,
        }}>
        <Show when={props.onBack}>
          {(onBack) => (
            <button class={`${styles.nav} ${styles.back}`} onClick={() => onBack()()} aria-label='Volver'>
              <svg viewBox='0 0 24 24' width='18' height='18' aria-hidden='true'>
                <path
                  d='M15 5l-7 7 7 7'
                  fill='none'
                  stroke='currentColor'
                  stroke-width='2'
                  stroke-linecap='round'
                  stroke-linejoin='round'
                />
              </svg>
            </button>
          )}
        </Show>

        <button class={`${styles.nav} ${styles.close}`} onClick={() => props.onClose?.()} aria-label='Cerrar'>
          <svg viewBox='0 0 24 24' width='18' height='18' aria-hidden='true'>
            <path d='M6 6l12 12M18 6L6 18' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' />
          </svg>
        </button>

        <div class={styles.body}>
          <SheetDragContext.Provider value={drag}>{props.children}</SheetDragContext.Provider>
        </div>
      </aside>
    </Show>
  );
}
