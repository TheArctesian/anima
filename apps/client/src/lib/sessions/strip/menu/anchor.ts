import type { Attachment } from 'svelte/attachments';

/** Where a menu sits under its control, in viewport pixels (`position: fixed`). */
export interface Placement {
  readonly left: number;
  readonly top: number;
}

/** The menu's corner at the bottom-left of `control`, so it opens below the control. */
export function below(control: DOMRectReadOnly): Placement {
  return { left: control.left, top: control.bottom };
}

/**
 * Moves the menu to the end of `document.body` while it is attached, so no scroll container
 * clips it or scrolls to show it (MOT-007, UI-009). The tab strip scrolls sideways.
 */
export const portal: Attachment<HTMLElement> = (node) => {
  document.body.append(node);
  return () => {
    node.remove();
  };
};
