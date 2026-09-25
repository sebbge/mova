import {createUniqueId} from 'solid-js';

// letter cut out with a mask
export default function ServiceIcon(props: {letter: string; size?: number}) {
  const maskId = `service-mask-${createUniqueId()}`;

  return (
    <svg width={props.size ?? 18} height={props.size ?? 18} viewBox='0 0 24 24' aria-hidden='true'>
      <mask id={maskId}>
        <rect width='24' height='24' fill='#fff' />
        <text
          x='12'
          y='12'
          text-anchor='middle'
          dominant-baseline='central'
          font-size='15'
          font-weight='700'
          font-family='var(--font)'
          fill='#000'>
          {props.letter}
        </text>
      </mask>
      <circle cx='12' cy='12' r='11' fill='currentColor' mask={`url(#${maskId})`} />
    </svg>
  );
}
