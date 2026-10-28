import {For, type JSX} from 'solid-js';
import styles from './SheetTabs.module.css';

export default function SheetTabs<T extends {id: string; disabled?: boolean}>(props: {
  label: string;
  tabs: T[];
  active: string;
  tab: (tab: T) => JSX.Element;
  onSelect?: (id: string) => void;
}) {
  // roving tabindex
  function walk(e: KeyboardEvent & {currentTarget: HTMLDivElement}) {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    const enabled = props.tabs.filter((t) => !t.disabled);
    const i = enabled.findIndex((t) => t.id === props.active);
    const next = enabled[(i + step + enabled.length) % enabled.length];
    if (!next) return;
    e.preventDefault();
    props.onSelect?.(next.id);
    const list = e.currentTarget;
    requestAnimationFrame(() => list.querySelector<HTMLElement>('[aria-selected="true"]')?.focus());
  }

  return (
    <div class={styles.tabs} role='tablist' aria-label={props.label} tabindex='-1' onKeyDown={walk}>
      <For each={props.tabs}>
        {(t) => (
          <button
            role='tab'
            classList={{[styles.active]: t.id === props.active}}
            disabled={t.disabled}
            aria-selected={t.id === props.active}
            tabindex={t.id === props.active ? 0 : -1}
            onClick={() => props.onSelect?.(t.id)}>
            {props.tab(t)}
          </button>
        )}
      </For>
    </div>
  );
}
