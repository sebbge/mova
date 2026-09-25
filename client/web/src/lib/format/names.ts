// no %20 in shared links
export function lineSlug(line: string): string {
  return line.replace(/ /g, '_');
}

export function lineFromSlug(slug: string): string {
  return slug.replace(/_/g, ' ');
}

export function rawId(ref: string): string {
  const i = ref.indexOf('-');
  return i === -1 ? ref : ref.slice(i + 1);
}
