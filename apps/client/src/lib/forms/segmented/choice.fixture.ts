// Test support: reading and driving a labelled `Segmented` choice the way the tests once drove a <select>.
import { flushSync } from 'svelte';

export interface Choice {
  /** The options, in order, as the radios that show them. */
  readonly options: readonly HTMLButtonElement[];
  /** The chosen option's value. */
  readonly value: string;
  /** Clicks the option with this value. */
  pick(value: string): void;
}

export function choiceOf(target: HTMLElement, label: string): Choice {
  const group = target.querySelector<HTMLElement>(`[role="radiogroup"][aria-label="${label}"]`);
  if (group === null) throw new Error(`no ${label}`);
  const radios = (): HTMLButtonElement[] => [...group.querySelectorAll<HTMLButtonElement>('[role="radio"]')];
  return {
    get options() {
      return radios();
    },
    get value() {
      return radios().find((radio) => radio.getAttribute('aria-checked') === 'true')?.dataset['value'] ?? '';
    },
    pick(value: string) {
      radios()
        .find((radio) => radio.dataset['value'] === value)
        ?.click();
      flushSync();
    },
  };
}
