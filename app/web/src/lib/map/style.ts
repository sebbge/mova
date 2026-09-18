const BASEMAP = {
  light: 'https://tiles.openfreemap.org/styles/positron',
  dark: 'https://tiles.openfreemap.org/styles/dark',
} as const;

export function basemapStyle(dark: boolean): string {
  return dark ? BASEMAP.dark : BASEMAP.light;
}
