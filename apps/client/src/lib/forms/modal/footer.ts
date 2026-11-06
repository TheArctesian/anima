import { getContext, setContext, type Snippet } from 'svelte';

/**
 * A modal's footer, below its scrolling body: a form inside the modal shows its actions there, so they
 * stay in view and never move while the body scrolls or a section opens. A modal holds one form: the
 * footer shows the actions of the form that showed them last.
 */
interface ModalFooter {
  /** Shows `actions` in the footer; the returned function takes them out again. */
  readonly show: (actions: Snippet) => () => void;
}

const key = Symbol('modal footer');

/** Called by the modal while it initialises, so the content rendered inside it finds the footer. */
export function offerFooter(footer: ModalFooter): void {
  setContext(key, footer);
}

/** The footer of the modal around the caller; undefined outside a modal, where actions stay in place. */
export function modalFooter(): ModalFooter | undefined {
  return getContext<ModalFooter | undefined>(key);
}
