import type { Snippet } from 'svelte';
import type { IconName } from '$lib/icons/icons';

/** One label and its value in a key-value list. */
export interface Pair {
  readonly label: string;
  readonly value: string;
  /** The icon that leads the label (UI-026, UR-7). */
  readonly icon?: IconName;
  /** Machine text (an address, a fingerprint): the monospace font, wrapping at any character. */
  readonly mono?: boolean;
  /** A slot for a copy button, drawn after the value. */
  readonly copy?: Snippet;
}
