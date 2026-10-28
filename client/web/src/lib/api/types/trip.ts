export type Payment = 'card' | 'cash';
export type Category = 'common' | 'student_a' | 'student_b' | 'retiree_a' | 'retiree_b';
export type TravelMode = 'transit' | 'walk';

export interface TripStop {
  id: string;
  name: string;
  lat: number;
  lng: number;
}

export type TripPlace = TripStop | {lat: number; lng: number};

export interface Ticket {
  product: string;
  name: string;
  price: number;
  legs: number[];
}

export interface Fare {
  currency: string | null;
  total: number | null;
  complete: boolean;
  tickets: Ticket[];
}

export interface WalkLeg {
  mode: 'walk';
  from: TripPlace;
  to: TripPlace;
  departure: string;
  arrival: string;
  duration_s: number;
  distance_m: number;
}

export interface BusLeg {
  mode: 'bus';
  provider: string;
  line: string;
  headsign: string | null;
  route_id: string;
  shape_id: string | null;
  from: TripStop;
  to: TripStop;
  departure: string;
  arrival: string;
  stops: number;
  interpolated: boolean;
  path: [number, number][];
}

export type Leg = WalkLeg | BusLeg;

export interface Itinerary {
  departure: string;
  arrival: string;
  duration_s: number;
  transfers: number;
  walk_m: number;
  fare: Fare;
  legs: Leg[];
}
