import {onCleanup} from 'solid-js';

type Frames = Keyframe[] | ((el: HTMLElement) => Keyframe[]);

const EASE_OUT = 'cubic-bezier(0.23, 1, 0.32, 1)';
const EASE_LEAVE = 'cubic-bezier(0.33, 1, 0.68, 1)';
const FADE: Keyframe[] = [{opacity: 0}, {opacity: 1}];

export const fly = (y: string, opacity = 0): Keyframe[] => [
  {translate: `0 ${y}`, opacity},
  {translate: '0 0', opacity: 1},
];

export const slide: Frames = (el) => [
  {height: '0px', overflow: 'hidden'},
  {height: `${el.scrollHeight}px`, overflow: 'hidden'},
];

// waapi skips the css reduced-motion rule
function still(): boolean {
  return matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function framesOf(frames: Frames, el: HTMLElement): Keyframe[] {
  return typeof frames === 'function' ? frames(el) : frames;
}

// call from a ref, a frozen copy plays the exit
export function transition(el: HTMLElement, frames: Frames, ms: {enter: number; leave: number}) {
  queueMicrotask(() => {
    const reduced = still();
    el.animate(reduced ? FADE : framesOf(frames, el), {duration: reduced ? 150 : ms.enter, easing: EASE_OUT});
  });

  onCleanup(() => {
    if (!el.isConnected) return;
    const reduced = still();
    const ghost = el.cloneNode(true) as HTMLElement;
    ghost.inert = true;
    ghost.removeAttribute('id');
    el.after(ghost);
    ghost
      .animate((reduced ? FADE : framesOf(frames, el)).toReversed(), {
        duration: reduced ? 100 : ms.leave,
        easing: EASE_LEAVE,
        fill: 'forwards',
      })
      .finished.finally(() => ghost.remove());
  });
}
