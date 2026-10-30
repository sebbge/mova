import BusIcon from './BusIcon';
import styles from './StopIcon.module.css';

export default function StopIcon(props: {size?: number}) {
  return (
    <span class={styles.pin} style={{'--pin-size': `${props.size ?? 36}px`}} aria-hidden='true'>
      <BusIcon />
    </span>
  );
}
