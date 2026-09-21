// points right, rotated along the line
export function chevronImage(size = 28): ImageData | null {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  const c = size / 2;
  const r = size * 0.32;

  ctx.beginPath();
  ctx.moveTo(c - r * 0.6, c - r);
  ctx.lineTo(c + r * 0.7, c);
  ctx.lineTo(c - r * 0.6, c + r);

  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';

  // light outline
  ctx.strokeStyle = 'rgba(255,255,255,0.9)';
  ctx.lineWidth = size * 0.28;
  ctx.stroke();

  ctx.strokeStyle = brand();
  ctx.lineWidth = size * 0.17;
  ctx.stroke();

  return ctx.getImageData(0, 0, size, size);
}

function brand(): string {
  return getComputedStyle(document.documentElement).getPropertyValue('--brand').trim() || '#00b360';
}
