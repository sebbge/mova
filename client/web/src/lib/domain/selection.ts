// selected stop + its shelter
export interface StopSelection {
  id: string | null;
  siblings: readonly string[];
}

export const isSelected = (sel: StopSelection, id: string) => id === sel.id || sel.siblings.includes(id);
