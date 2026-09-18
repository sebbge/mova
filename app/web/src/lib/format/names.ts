const LOWERCASE = new Set(['y', 'e', 'de', 'del', 'la', 'las', 'el', 'los', 'a', 'al', 'en']);

// gtfs over-capitalizes particles
export function stopName(raw: string): string {
  return raw
    .split(/\s+/)
    .map((word, i) => {
      const lower = word.toLowerCase();
      return i > 0 && LOWERCASE.has(lower) ? lower : word;
    })
    .join(' ');
}
