import type {Service} from '~/lib/domain/providers';
import BusIcon from '~/lib/ui/icons/BusIcon';
import MapTip from './MapTip';
import styles from './StopTooltip.module.css';

export interface StopHover {
  name: string;
  services: Service[];
  x: number; // px in the map container
  y: number;
}

// split diagonally for two services
export default function StopTooltip(props: StopHover) {
  const first = () => props.services[0] ?? 'neutral';
  const second = () => props.services[1] ?? first();

  return (
    <div class={styles.anchor} style={{left: `${props.x}px`, top: `${props.y}px`}}>
      <MapTip
        text={props.name}
        lead={
          <span
            class={styles.icon}
            style={{
              '--first': `var(--service-${first()}-pin)`,
              '--second': `var(--service-${second()}-pin)`,
              '--ring': `var(--service-${first()}-outline)`,
            }}>
            <BusIcon />
          </span>
        }
      />
    </div>
  );
}
