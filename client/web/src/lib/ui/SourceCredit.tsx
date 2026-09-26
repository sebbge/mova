import {Show} from 'solid-js';
import {providerInfo} from '~/lib/domain/providers';
import styles from './SourceCredit.module.css';

// only with a registered attribution
export default function SourceCredit(props: {provider: string | undefined; tinted?: boolean}) {
  const attribution = () => (props.provider ? providerInfo(props.provider).attribution : undefined);

  return (
    <Show when={attribution()}>
      {(a) => (
        <p class={styles.credit} classList={{[styles.tinted]: Boolean(props.tinted)}}>
          {a().label}:{' '}
          <a href={a().href} target='_blank' rel='noopener noreferrer'>
            {a().source}
          </a>{' '}
          ({a().license})
        </p>
      )}
    </Show>
  );
}
