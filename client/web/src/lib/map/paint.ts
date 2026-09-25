import type {Service} from '~/lib/domain/providers';

// css tokens as rgb()
const PIN_FALLBACK: Record<Service, string> = {
  urban: '#2d487f',
  suburban: '#2f7a63',
  inter: '#bc6b23',
};

let probe: CanvasRenderingContext2D | null = null;

export function token(name: string, fallback: string): string {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  if (!raw) return fallback;

  if (!probe) {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    probe = canvas.getContext('2d', {willReadFrequently: true});
    if (probe) probe.globalCompositeOperation = 'copy';
  }
  if (!probe) return raw;

  probe.fillStyle = raw;
  probe.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = probe.getImageData(0, 0, 1, 1).data;
  return a === 255 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${(a / 255).toFixed(3)})`;
}

export function brand(): string {
  return token('--brand', '#2fb152');
}

export function outline(): string {
  return token('--map-outline', '#ffffff');
}

export function pinColor(service: Service): string {
  return token(`--service-${service}`, PIN_FALLBACK[service]);
}

export function pinOutline(service: Service): string {
  return token(`--service-${service}-outline`, outline());
}
