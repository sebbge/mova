import {For, Show} from 'solid-js';
import type {Coverage} from './coverage';
import styles from './DepartmentShape.module.css';
import {DEPARTMENT_PATHS} from './departments';

// padded, so the neighbors show
function frame(path: string): string {
  const n = path.match(/-?\d+(\.\d+)?/g)?.map(Number) ?? [];
  const xs = n.filter((_, i) => i % 2 === 0);
  const ys = n.filter((_, i) => i % 2 === 1);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const pad = Math.max(x1 - x0, y1 - y0) * 0.15;
  return [x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad].map((v) => v.toFixed(1)).join(' ');
}

export default function DepartmentShape(props: {id: string; coverage: Coverage}) {
  return (
    <svg class={styles.shape} viewBox={frame(DEPARTMENT_PATHS[props.id] ?? '')} aria-hidden='true'>
      <For each={Object.entries(DEPARTMENT_PATHS)}>
        {([other, d]) => (
          <Show when={other !== props.id}>
            <path d={d} />
          </Show>
        )}
      </For>
      <path class={styles.own} d={DEPARTMENT_PATHS[props.id]} style={{'--fill': `var(--coverage-${props.coverage})`}} />
    </svg>
  );
}
