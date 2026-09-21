// no %20 in shared links
export function lineSlug(line: string): string {
  return line.replace(/ /g, '_');
}

export function lineFromSlug(slug: string): string {
  return slug.replace(/_/g, ' ');
}
