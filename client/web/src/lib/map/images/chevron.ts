import {token} from '~/lib/map/paint';

export const CHEVRON_PIXEL_RATIO = 2;

// points right, rotated along the line
export function chevronImage(size = 32): ImageData | null {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  const c = size / 2;
  const arm = size * 0.26;

  ctx.beginPath();
  ctx.moveTo(c - arm * 0.62, c - arm);
  ctx.lineTo(c + arm * 0.62, c);
  ctx.lineTo(c - arm * 0.62, c + arm);

  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.strokeStyle = token('--brand-contrast', '#ffffff');
  ctx.lineWidth = size * 0.2;
  ctx.stroke();

  return ctx.getImageData(0, 0, size, size);
}
