import styles from '~/lib/ui/Header.module.css';
import {BackLink, TripQuery} from '~/lib/ui/HeaderParts';
import ThemeToggle from '~/lib/ui/ThemeToggle';

// inside the island, search opens in place
export default function MapHeader(props: {destination?: string; onSearch: () => void}) {
  return (
    <header class={styles.header}>
      <div class={styles.inner}>
        <BackLink href='/' />
        <TripQuery destination={props.destination} onSearch={() => props.onSearch()} />
        <ThemeToggle />
      </div>
    </header>
  );
}
