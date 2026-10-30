import {For} from 'solid-js';
import styles from './RouteSkeleton.module.css';

// uneven widths read as loading
const WIDTHS = [58, 74, 46, 81, 63, 52, 70, 44]; // %

export default function RouteSkeleton(props: {rows?: number}) {
  return (
    <ul class={styles.root} aria-hidden='true'>
      <For each={Array.from({length: props.rows ?? 8}, (_, i) => WIDTHS[i % WIDTHS.length])}>
        {(width) => (
          <li>
            <span class={`${styles.node} shimmer`} />
            <span class={`${styles.name} shimmer`} style={{width: `${width}%`}} />
            <span class={`${styles.at} shimmer`} />
          </li>
        )}
      </For>
    </ul>
  );
}
