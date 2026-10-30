import type {ExpressionSpecification} from 'maplibre-gl';
import type {Service} from '~/lib/domain/providers';
import {companySlug} from '~/lib/ui/palette';

// css tokens as rgb()
const PIN_FALLBACK: Record<Service, string> = {
  urban: '#2d487f',
  suburban: '#2f7a63',
  inter: '#bc6b23',
  neutral: '#b4b9c1',
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

// theme color with halo
export function mapText(): {color: string; halo: string} {
  return {color: token('--text-primary', '#1b1d22'), halo: token('--surface', '#ffffff')};
}

// gray without a company
export function routeColor(company: string | undefined): string {
  const slug = companySlug(company) ?? 'unknown';
  return token(`--company-${slug}-pin`, token(`--company-${slug}`, '#8a8f98'));
}

export function outline(): string {
  return token('--map-outline', '#ffffff');
}

export function pinColor(service: Service): string {
  return token(`--service-${service}-pin`, PIN_FALLBACK[service]);
}

export function pinOutline(service: Service): string {
  return token(`--service-${service}-outline`, outline());
}

export function tripColor(kind: 'walk' | 'origin' | 'destination'): string {
  if (kind === 'walk') return token('--walk', '#7a7a7a');
  return token(`--trip-${kind}`, kind === 'origin' ? '#2e9d62' : '#d8453a');
}

export function serviceColor(): ExpressionSpecification {
  return [
    'match',
    ['get', 'service'],
    'inter',
    pinColor('inter'),
    'suburban',
    pinColor('suburban'),
    'neutral',
    pinColor('neutral'),
    pinColor('urban'),
  ];
}
