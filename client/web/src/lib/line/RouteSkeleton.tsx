import {For} from 'solid-js';
import styles from './RouteSkeleton.module.css';

const WIDTHS = [58, 74, 46, 81, 63, 52, 70, 44]; // %

export default function RouteSkeleton(props: {rows?: number}) {
  return (
    <ul class={styles.root} aria-hidden='true'>
      <For each={Array.from({length: props.rows ?? 8}, (_, i) => WIDTHS[i % WIDTHS.length])}>
        {(width) => (
          <li>
            <span class={`${styles.id} shimmer`} />
            <span class={`${styles.name} shimmer`} style={{width: `${width}%`}} />
          </li>
        )}
      </For>
    </ul>
  );
}
