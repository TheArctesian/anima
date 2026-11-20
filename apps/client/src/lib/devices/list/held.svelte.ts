import type { Attachment } from 'svelte/attachments';
import { heldOrder } from './held';
import type { ListEntry } from './look';

/**
 * The rows of a live list in the order the operator sees. While the pointer is on the list, a change that only moves
 * rows (a live status) waits, so the row under the pointer is the row it was; the rows move to their new places when the
 * pointer leaves. A change of `basis` (the sort or the filter that the operator chose) applies at once (5.7.3, Reorder; UR-7 Phase F).
 * Call during component init; attach `hold` to the list's element.
 */
export class HeldOrder<T> {
  #pointerIn = $state(false);
  #shown: readonly ListEntry<T>[] = [];
  #shownBasis: string | undefined;
  readonly #source: () => readonly ListEntry<T>[];
  readonly #basis: () => string;

  constructor(source: () => readonly ListEntry<T>[], basis: () => string) {
    this.#source = source;
    this.#basis = basis;
  }

  /** The entries to render, in the held order while the pointer is on the list. */
  readonly items: readonly ListEntry<T>[] = $derived.by(() => {
    const next = this.#source();
    const basis = this.#basis();
    const kept = this.#pointerIn && basis === this.#shownBasis ? heldOrder(this.#shown, next) : null;
    const shown = kept ?? next;
    this.#shown = shown;
    this.#shownBasis = basis;
    return shown;
  });

  /** Follows the pointer in and out of the list element. */
  readonly hold: Attachment<HTMLElement> = (element) => {
    const enter = (): void => {
      this.#pointerIn = true;
    };
    const leave = (): void => {
      this.#pointerIn = false;
    };
    element.addEventListener('pointerenter', enter);
    element.addEventListener('pointerleave', leave);
    element.addEventListener('pointercancel', leave);
    return () => {
      element.removeEventListener('pointerenter', enter);
      element.removeEventListener('pointerleave', leave);
      element.removeEventListener('pointercancel', leave);
    };
  };
}
