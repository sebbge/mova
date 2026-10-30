import {For, Show} from 'solid-js';
import type {Bus} from '~/lib/api/types';
import MetaIcon, {type MetaKind} from './MetaIcon';
import styles from './MetaIcons.module.css';

// in reading order
export function metaTags(bus: Bus | undefined): {kind: MetaKind; text: string}[] {
  const tags: [MetaKind, string | undefined][] = [
    ['access', bus?.access],
    ['emissions', bus?.emissions],
    ['thermal', bus?.thermal_comfort],
  ];
  return tags.flatMap(([kind, text]) => (text ? [{kind, text}] : []));
}

export default function MetaIcons(props: {bus?: Bus}) {
  return (
    <Show when={metaTags(props.bus).length > 0}>
      <span class={styles.meta}>
        <For each={metaTags(props.bus)}>
          {(tag) => (
            <span style={{color: `var(--meta-${tag.kind})`}}>
              <MetaIcon kind={tag.kind} label={tag.text} />
            </span>
          )}
        </For>
      </span>
    </Show>
  );
}
