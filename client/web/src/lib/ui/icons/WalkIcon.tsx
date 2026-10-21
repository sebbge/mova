export default function WalkIcon(props: {size?: number}) {
  return (
    <svg viewBox='0 0 24 24' width={props.size ?? 18} height={props.size ?? 18} aria-hidden='true'>
      <circle cx='13' cy='4' r='2' fill='currentColor' />
      <path
        d='M9.5 21l2-6 2.5 2.5V21M8 12.5l1.5-4.5 3.5 1 2 3 2.5 1M11 9l-1 6'
        fill='none'
        stroke='currentColor'
        stroke-width='1.8'
        stroke-linecap='round'
        stroke-linejoin='round'
      />
    </svg>
  );
}
