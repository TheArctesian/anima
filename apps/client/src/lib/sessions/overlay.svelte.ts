import { untrack } from 'svelte';

/** How many menus, popovers and modals are open over the panes right now. */
let covering = $state(0);

/** Whether anything is open over the panes, so a native web view must hide (MAT-008, WEB-005). */
export function overlayCovers(): boolean {
  return covering > 0;
}

/**
 * Call while a component initialises: the pane is covered for as long as `isOpen()` is true. The count goes down
 * when it turns false and when the component is destroyed, so it cannot stick.
 */
export function coverWhile(isOpen: () => boolean): void {
  $effect(() => {
    if (!isOpen()) return;
    untrack(() => (covering += 1));
    return () => {
      covering -= 1;
    };
  });
}
