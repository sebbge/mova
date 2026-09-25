import {createSignal, onCleanup, onMount, Show, type JSX} from 'solid-js';
import {SheetDragContext, type SheetDrag} from './context';
import styles from './Sheet.module.css';

const SNAPS = [0.22, 0.48, 0.88]; // share of screen height: peek, half, full
const DRAG_THRESHOLD = 0.06; // share of screen height

export default function Sheet(props: {
  open: boolean;
  onBack?: () => void; // only with a previous step
  onClose?: () => void;
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
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);

    const move = (ev: PointerEvent) => setDragOffset(ev.clientY - startY);
    const up = () => {
      target.releasePointerCapture(e.pointerId);
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);

      const threshold = innerHeight * DRAG_THRESHOLD;
      const next = snap() + (dragOffset() > threshold ? -1 : dragOffset() < -threshold ? 1 : 0);
      if (next < 0) props.onClose?.();
      setSnap(clampSnap(next));
      setDragging(false);
      setDragOffset(0);
    };

    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
  }

  const drag: SheetDrag = {
    start,
    get snap() {
      return snap();
    },
    max: SNAPS.length - 1,
  };

  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') props.onClose?.();
      if (e.key === 'ArrowUp') setSnap(clampSnap(snap() + 1));
      if (e.key === 'ArrowDown') setSnap(clampSnap(snap() - 1));
    };
    addEventListener('keydown', onKeyDown);
    onCleanup(() => removeEventListener('keydown', onKeyDown));
  });

  return (
    <Show when={props.open}>
      <aside
        class={styles.sheet}
        classList={{[styles.dragging]: dragging()}}
        style={{
          '--sheet-nav-start': props.onBack ? '44px' : undefined,
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
