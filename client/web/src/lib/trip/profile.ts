import {createRoot, createSignal} from 'solid-js';
import type {Category, Payment} from '~/lib/api/types';

const KEY = 'mova.fare-profile';

export const PAYMENTS: {id: Payment; label: string}[] = [
  {id: 'card', label: 'Tarjeta STM'},
  {id: 'cash', label: 'Efectivo'},
];

export const CATEGORIES: {id: Category; label: string}[] = [
  {id: 'common', label: 'Común'},
  {id: 'student_a', label: 'Estudiante A'},
  {id: 'student_b', label: 'Estudiante B'},
  {id: 'retiree_a', label: 'Jubilado A'},
  {id: 'retiree_b', label: 'Jubilado B'},
];

// remembered across visits
class FareProfile {
  readonly #payment = createRoot(() => createSignal<Payment>('card'));
  readonly #category = createRoot(() => createSignal<Category>('common'));

  constructor() {
    if (typeof localStorage === 'undefined') return;
    try {
      const saved = JSON.parse(localStorage.getItem(KEY) ?? 'null');
      if (PAYMENTS.some((p) => p.id === saved?.payment)) this.#payment[1](saved.payment);
      if (CATEGORIES.some((c) => c.id === saved?.category)) this.#category[1](saved.category);
    } catch {
      // unreadable, defaults
    }
  }

  get payment(): Payment {
    return this.#payment[0]();
  }

  get category(): Category {
    return this.#category[0]();
  }

  get label(): string {
    const category = CATEGORIES.find((c) => c.id === this.category)?.label ?? '';
    return this.payment === 'cash' ? `${category} · Efectivo` : category;
  }

  set(payment: Payment, category: Category) {
    this.#payment[1](payment);
    this.#category[1](category);
    try {
      localStorage.setItem(KEY, JSON.stringify({payment, category}));
    } catch {
      // private mode, this page only
    }
  }
}

export const fareProfile = new FareProfile();
