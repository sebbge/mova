import type {Service} from '~/lib/domain/providers';

// one per department with live data
export const DEMO_STOPS = [
  {id: 'stm-3179', label: 'Montevideo'},
  {id: 'codesa-934', label: 'Punta del Este'},
  {id: 'solantigua-8', label: 'Colonia'},
] as const;

export interface DemoStop {
  id: string;
  label: string;
  kicker: string;
  name: string;
  place: string;
  service: Service;
}
