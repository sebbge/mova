import {Show, type JSX} from 'solid-js';
import {useSheetDrag} from './context';
import styles from './SheetHeader.module.css';

export default function SheetHeader(props: {
  kicker?: string;
  title: string;
  subtitle?: string;
  lead?: JSX.Element;
  loading?: boolean; // bars until known
}) {
  const drag = useSheetDrag();
  const shimmer = () => ({shimmer: Boolean(props.loading)});

  return (
    <header
      class={styles.header}
      classList={{[styles.drag]: drag !== undefined}}
      role={drag ? 'slider' : undefined}
      tabindex={drag ? 0 : undefined}
      aria-label={drag ? 'Altura del panel' : undefined}
      aria-valuemin={drag ? 0 : undefined}
      aria-valuenow={drag?.snap}
      aria-valuemax={drag?.max}
      onPointerDown={(e) => drag?.start(e)}
      onKeyDown={(e) => drag?.key(e)}>
      {props.lead}
      <div class={styles.meta} aria-busy={props.loading || undefined}>
        <Show when={props.kicker !== undefined}>
          <span class={styles.kicker} classList={shimmer()}>
            {props.loading ? '' : props.kicker}
          </span>
        </Show>
        <h2 class={styles.title} classList={shimmer()}>
          {props.loading ? '' : props.title}
        </h2>
        <Show when={props.loading || props.subtitle}>
          <span class={styles.subtitle} classList={shimmer()}>
            {props.loading ? '' : props.subtitle}
          </span>
        </Show>
      </div>
    </header>
  );
}
