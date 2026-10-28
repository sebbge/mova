import type {Feature, FeatureCollection, LineString} from 'geojson';

type LngLat = [number, number];

const M_PER_DEG = 111_320;
const NEAR_M = 100; // m
const LOOKAHEAD_M = 1500; // m

// stops projected onto the line, cut stop to stop
export class RouteTrack {
  readonly points: LngLat[] = [];
  readonly #coords: LngLat[];
  readonly #distances: number[] = [0]; // m at each vertex
  readonly #along: number[] = []; // m at each stop

  constructor(line: LineString, stops: readonly {lat: number; lng: number}[]) {
    this.#coords = line.coordinates as LngLat[];
    const lat0 = this.#coords[0]?.[1] ?? 0;
    const kx = M_PER_DEG * Math.cos((lat0 * Math.PI) / 180);
    const xy = this.#coords.map(([lng, lat]) => [lng * kx, lat * M_PER_DEG] as const);

    for (let i = 1; i < xy.length; i++) {
      this.#distances.push(this.#distances[i - 1] + Math.hypot(xy[i][0] - xy[i - 1][0], xy[i][1] - xy[i - 1][1]));
    }

    let from = 0;
    for (const stop of stops) {
      const best = this.#nearest(xy, [stop.lng * kx, stop.lat * M_PER_DEG], from);
      from = best.segment;
      this.#along.push(best.along);
      this.points.push(this.#at(best.along));
    }
  }

  // trimmed margin(i) m around stop i
  sections(margin: (i: number) => number): FeatureCollection<LineString> {
    const total = this.#distances[this.#distances.length - 1] ?? 0;
    const bounds: [number, number][] = [];
    let start = 0;
    this.#along.forEach((along, i) => {
      bounds.push([start, along - margin(i)]);
      start = along + margin(i);
    });
    bounds.push([start, total]);

    return {
      type: 'FeatureCollection',
      features: bounds.filter(([a, b]) => b - a > 0.5).map(([a, b]) => this.#slice(a, b)),
    };
  }

  // in route order, a street run twice passes the stop twice
  #nearest(xy: readonly (readonly [number, number])[], p: readonly [number, number], from: number) {
    let best = {dist: Infinity, along: this.#distances[from] ?? 0, segment: from};
    for (let i = from; i < xy.length - 1; i++) {
      if (best.dist < NEAR_M && this.#distances[i] > best.along + LOOKAHEAD_M) break;
      const [ax, ay] = xy[i];
      const dx = xy[i + 1][0] - ax;
      const dy = xy[i + 1][1] - ay;
      const len2 = dx * dx + dy * dy;
      const t = len2 === 0 ? 0 : Math.max(0, Math.min(1, ((p[0] - ax) * dx + (p[1] - ay) * dy) / len2));
      const dist = Math.hypot(ax + t * dx - p[0], ay + t * dy - p[1]);
      if (dist < best.dist) best = {dist, along: this.#distances[i] + t * Math.sqrt(len2), segment: i};
    }
    return best;
  }

  #at(along: number): LngLat {
    const i = this.#segmentAt(along);
    const span = this.#distances[i + 1] - this.#distances[i];
    const t = span === 0 ? 0 : (along - this.#distances[i]) / span;
    const [a, b] = [this.#coords[i], this.#coords[i + 1] ?? this.#coords[i]];
    return [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
  }

  #segmentAt(along: number): number {
    let i = 0;
    while (i < this.#distances.length - 2 && this.#distances[i + 1] < along) i++;
    return i;
  }

  #slice(start: number, end: number): Feature<LineString> {
    const coordinates: LngLat[] = [this.#at(start)];
    for (let i = 0; i < this.#distances.length; i++) {
      if (this.#distances[i] > start && this.#distances[i] < end) coordinates.push(this.#coords[i]);
    }
    coordinates.push(this.#at(end));
    return {type: 'Feature', geometry: {type: 'LineString', coordinates}, properties: {}};
  }
}
