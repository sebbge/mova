import {createContext, useContext} from 'solid-js';

// each header is the drag surface
export interface SheetDrag {
  start: (e: PointerEvent) => void;
  readonly snap: number;
  readonly max: number;
}

export const SheetDragContext = createContext<SheetDrag>();

export function useSheetDrag(): SheetDrag | undefined {
  return useContext(SheetDragContext); // undefined outside a sheet
}
