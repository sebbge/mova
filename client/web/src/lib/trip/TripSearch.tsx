import {createSignal, onCleanup, onMount, Show} from 'solid-js';
import {createStore} from 'solid-js/store';
import type {TravelMode} from '~/lib/api/types';
import {fly, transition} from '~/lib/ui/motion';
import type {Endpoint} from './endpoint';
import SearchResults from './SearchResults';
import TripOptions from './TripOptions';
import type {Place} from './TripFeed';
import styles from './TripSearch.module.css';

type Field = 'a' | 'b';

export default function TripSearch(props: {
  a: Place | null;
  b: Place | null;
  field: Field;
  near?: {lat: number; lng: number};
  mode: TravelMode;
  at: string | null;
  onPoint?: (field: Field, endpoint: Endpoint, label: string) => void;
  onPick?: (field: Field) => void;
  onSwap?: () => void;
  onAt?: (at: string | null) => void;
  onClose?: () => void;
}) {
  const [active, setActive] = createSignal<Field>('b');
  const [terms, setTerms] = createStore<Record<Field, string>>({a: '', b: ''});
  const inputs: Partial<Record<Field, HTMLInputElement>> = {};
  let panel!: HTMLDivElement;
  const term = () => terms[active()].trim();

  onMount(() => {
    setActive(props.field);
    const opener = document.activeElement as HTMLElement | null;
    inputs[props.field]?.focus();
    onCleanup(() => opener?.focus());
  });

  // focus trap, escape closes the panel only
  function trap(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      props.onClose?.();
      return;
    }
    if (e.key !== 'Tab') return;
    const focusable = [...panel.querySelectorAll<HTMLElement>('button:not(:disabled), input')];
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  }

  function type(which: Field, value: string) {
    setTerms(which, value);
    setActive(which);
  }

  function choose(endpoint: Endpoint, label: string) {
    const which = active();
    setTerms(which, '');
    props.onPoint?.(which, endpoint, label);
    if (which === 'a' && !props.b) {
      setActive('b');
      inputs.b?.focus();
    }
  }

  const Input = (p: {field: Field; label: string; placeholder: string}) => (
    <div class={styles.field}>
      <span class={`${styles.dot} ${p.field === 'a' ? styles.origin : styles.destination}`} aria-hidden='true'>
        {p.field.toUpperCase()}
      </span>
      <input
        ref={(el) => (inputs[p.field] = el)}
        value={terms[p.field]}
        onInput={(e) => type(p.field, e.currentTarget.value)}
        onFocus={() => setActive(p.field)}
        placeholder={p.placeholder}
        aria-label={p.label}
        type='search'
      />
    </div>
  );

  return (
    <div
      ref={(el) => {
        panel = el;
        transition(el, fly('8px'), {enter: 200, leave: 120});
      }}
      class={styles.panel}
      role='dialog'
      aria-modal='true'
      aria-label='Planificar viaje'
      tabindex='-1'
      on:keydown={trap}>
      <div class={styles.fields}>
        <Input field='a' label='Origen' placeholder={props.a?.label ?? 'Origen'} />
        <Input field='b' label='Destino' placeholder={props.b?.label ?? '¿A dónde vas?'} />
        <button class={styles.swap} onClick={() => props.onSwap?.()} aria-label='Invertir origen y destino'>
          <svg viewBox='0 0 24 24' width='18' height='18' aria-hidden='true'>
            <path
              d='M8 4v16M8 4L4 8M8 4l4 4M16 20V4M16 20l-4-4M16 20l4-4'
              fill='none'
              stroke='currentColor'
              stroke-width='2'
              stroke-linecap='round'
              stroke-linejoin='round'
            />
          </svg>
        </button>
      </div>

      <SearchResults term={term()} near={props.near} onChoose={choose} onPick={() => props.onPick?.(active())} />
      <Show when={!term()}>
        <TripOptions mode={props.mode} at={props.at} onAt={props.onAt} />
      </Show>

      <button class={styles.done} onClick={() => props.onClose?.()}>
        Listo
      </button>
    </div>
  );
}
