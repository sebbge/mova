import busSvg from '~/lib/assets/bus.svg?raw';
import type {Service} from '~/lib/domain/providers';
import {outline, pinColor, pinOutline} from '~/lib/map/paint';

const [vbX, vbY, vbW, vbH] = (busSvg.match(/viewBox="([^"]+)"/)?.[1] ?? '0 0 1 1').split(/\s+/).map(Number);
const busPath = new Path2D(busSvg.match(/ d="([^"]+)"/)?.[1] ?? '');

// drawn at 2x, or blurry on hidpi
export const STOP_PIN_PIXEL_RATIO = 2;

const DOT = {plain: {radius: 3.5, ring: 1.25}, selected: {radius: 5.75, ring: 2}}; // screen px

// close zoom, size in image px
export function stopPinImage(selected: boolean, services: readonly Service[], size = 80): ImageData | null {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext('2d');
  if (!ctx || !services[0]) return null;

  const margin = size * 0.12;
  const side = size - margin * 2;
  const shape = () => roundedRect(ctx, margin, margin, side, side, side * 0.26);

  // selected inverted
  byService(ctx, shape, margin, side, services, (service) => {
    ctx.fillStyle = selected ? outline() : pinColor(service);
    ctx.fill();
  });

  ctx.lineWidth = selected ? size * 0.07 : size * 0.038;
  byService(
    ctx,
    shape,
    margin,
    side,
    services,
    (service) => {
      ctx.strokeStyle = pinOutline(service);
      ctx.stroke();
    },
    ctx.lineWidth
  );

  const scale = (side * 0.62) / Math.max(vbW, vbH);
  byService(ctx, shape, margin, side, services, (service) => {
    ctx.save();
    ctx.translate(size / 2 - (vbW * scale) / 2, size / 2 - (vbH * scale) / 2);
    ctx.scale(scale, scale);
    ctx.translate(-vbX, -vbY);
    ctx.fillStyle = selected ? pinColor(service) : outline();
    ctx.fill(busPath);
    ctx.restore();
  });

  return ctx.getImageData(0, 0, size, size);
}

// far zoom
export function stopDotImage(selected: boolean, services: readonly Service[]): ImageData | null {
  const {radius, ring} = selected ? DOT.selected : DOT.plain;
  const inner = radius * STOP_PIN_PIXEL_RATIO;
  const outer = inner + ring * STOP_PIN_PIXEL_RATIO;
  const size = Math.ceil(outer * 2) + 2;

  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext('2d');
  if (!ctx || !services[0]) return null;

  const center = size / 2;
  const disc = (r: number) => () => {
    ctx.beginPath();
    ctx.arc(center, center, r, 0, Math.PI * 2);
  };
  const fill = selected ? pinOutline : pinColor;
  const edge = selected ? pinColor : pinOutline;

  for (const [r, color] of [
    [outer, edge],
    [inner, fill],
  ] as const) {
    byService(ctx, disc(r), center - r, r * 2, services, (service) => {
      ctx.fillStyle = color(service);
      ctx.fill();
    });
  }

  return ctx.getImageData(0, 0, size, size);
}

function byService(
  ctx: CanvasRenderingContext2D,
  shape: () => void,
  margin: number,
  side: number,
  [first, second]: readonly Service[],
  draw: (service: Service) => void,
  stroke = 0
): void {
  if (!first) return;
  if (!second) {
    shape();
    draw(first);
    return;
  }
  halved(
    ctx,
    shape,
    margin,
    side,
    (half) => {
      shape();
      draw(half === 'upper' ? first : second);
    },
    stroke
  );
}

// first service on top, as the sheet tabs
function halved(
  ctx: CanvasRenderingContext2D,
  shape: () => void,
  margin: number,
  side: number,
  draw: (half: 'upper' | 'lower') => void,
  stroke = 0
): void {
  // grown half a stroke, or the outline looks thinner
  const pad = stroke / 2;
  const start = margin - pad;
  const end = margin + side + pad;
  const halves: ['upper' | 'lower', [number, number][]][] = [
    [
      'upper',
      [
        [start, start],
        [end, start],
        [start, end],
      ],
    ],
    [
      'lower',
      [
        [end, start],
        [end, end],
        [start, end],
      ],
    ],
  ];

  for (const [half, points] of halves) {
    ctx.save();
    if (!stroke) {
      shape();
      ctx.clip(); // keeps the rounded corners
    }

    ctx.beginPath();
    for (const [x, y] of points) ctx.lineTo(x, y);
    ctx.closePath();
    ctx.clip();

    draw(half);
    ctx.restore();
  }
}

// 64 image px, 32 on screen
export function routeTerminalImage(letter: 'O' | 'D', color: string, selected: boolean, size = 64): ImageData | null {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  const ring = size * 0.09;
  const side = size - ring;
  const [fill, ink] = selected ? [color, outline()] : [outline(), color];

  roundedRect(ctx, ring / 2, ring / 2, side, side, side * 0.28);
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.lineWidth = ring;
  ctx.strokeStyle = ink;
  ctx.stroke();

  ctx.fillStyle = ink;
  ctx.font = `700 ${Math.round(size * 0.5)}px system-ui, sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(letter, size / 2, size / 2 + size * 0.03);

  return ctx.getImageData(0, 0, size, size);
}

function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
