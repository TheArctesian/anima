import type { Attachment } from 'svelte/attachments';
import { animate } from '$lib/motion/animate';
import { tweenMotion } from '$lib/motion/options';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';
import { flip, measure, type Places } from '$lib/motion/roles/flip';

/** The row's place one row above where it is: where a row that just arrived comes in from. */
function above(row: HTMLElement): DOMRectReadOnly {
  const { left, top, width, height } = row.getBoundingClientRect();
  return new DOMRectReadOnly(left, top - height, width, height);
}

/** The highlight (the row's `data-glow` layer) shows at once and fades over `duration-highlight`; reduced motion keeps it (5.7.3, New log row). */
function lightUp(row: HTMLElement, reduced: boolean): void {
  const glow = row.querySelector<HTMLElement>('[data-glow]');
  if (glow === null) return;
  animate(glow, { opacity: 1 }, { type: 'instant' });
  animate(glow, { opacity: 0 }, tweenMotion('highlight', 'exit', reduced));
}

/**
 * The new log row role (5.7.3): the row `arrived()` names comes in at the top while the rows below it move down by FLIP
 * (`spring-move`), and its highlight fades. Under reduced motion the rows change at once and the highlight remains.
 * Call during component init; attach the returned function to each row with its id.
 */
export function newRowMotion(
  arrived: () => string | null,
  preference: MotionPreference = motionPreference(),
): (id: string) => Attachment<HTMLElement> {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a registry read by effects, never rendered
  const rows = new Map<string, HTMLElement>();
  let before: Places | undefined;

  $effect.pre(() => {
    arrived();
    before = measure(rows.values());
  });

  $effect(() => {
    const id = arrived();
    const places = before;
    before = undefined;
    const row = id === null ? undefined : rows.get(id);
    if (row === undefined || places === undefined) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- passed once to flip, not state
    flip(new Map(places).set(row, above(row)), { preference });
    lightUp(row, preference.reduced);
  });

  return (id) => (row) => {
    rows.set(id, row);
    return () => {
      if (rows.get(id) === row) rows.delete(id);
    };
  };
}
