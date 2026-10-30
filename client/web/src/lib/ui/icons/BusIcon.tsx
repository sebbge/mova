import busSvg from '~/lib/assets/bus.svg?raw';

// the asset's path as markup
const VIEW_BOX = busSvg.match(/viewBox="([^"]+)"/)?.[1];
const PATH = busSvg.match(/ d="([^"]+)"/)?.[1];

export default function BusIcon() {
  return (
    <svg viewBox={VIEW_BOX} fill='currentColor' aria-hidden='true'>
      <path d={PATH} />
    </svg>
  );
}
