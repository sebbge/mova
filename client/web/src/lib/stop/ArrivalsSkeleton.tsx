import {For} from 'solid-js';
import styles from './ArrivalsSkeleton.module.css';

const WIDTHS = [72, 54, 83, 61, 45, 76, 58, 88, 49, 67, 79, 52]; // %, uneven

export default function ArrivalsSkeleton(props: {rows?: number}) {
  return (
    <ul class={styles.root} aria-hidden='true'>
      <For each={Array.from({length: props.rows ?? 14}, (_, i) => WIDTHS[i % WIDTHS.length])}>
        {(width) => (
          <li>
            <span class={`${styles.badge} shimmer`} />
            <span class={`${styles.destination} shimmer`} style={{width: `${width}%`}} />
            <span class={`${styles.eta} shimmer`} />
          </li>
        )}
      </For>
    </ul>
  );
}
