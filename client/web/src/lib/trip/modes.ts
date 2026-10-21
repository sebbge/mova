import type {TravelMode} from '~/lib/api/types';

export const MODES: {id: TravelMode | 'car' | 'bike'; label: string; ready: boolean}[] = [
  {id: 'transit', label: 'Transporte público', ready: true},
  {id: 'car', label: 'Vehículo', ready: false},
  {id: 'bike', label: 'Bicicleta', ready: false},
];
