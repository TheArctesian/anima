// Test support: finding and driving the sign-in page's fields.
import { flushSync } from 'svelte';
import { vi } from 'vitest';

export function input(target: HTMLElement, id: string): HTMLInputElement {
  const element = target.querySelector<HTMLInputElement>(`#${id}`);
  if (element === null) throw new Error(`no #${id}`);
  return element;
}

export function type(target: HTMLElement, id: string, text: string): void {
  const element = input(target, id);
  element.value = text;
  element.dispatchEvent(new Event('input', { bubbles: true }));
}

export async function submit(target: HTMLElement, formIndex: number): Promise<void> {
  target.querySelectorAll('form')[formIndex]?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => {
    flushSync();
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** The text of the message under a field, without its icon. */
export function problemOf(target: HTMLElement, id: string): string {
  const problem = target.querySelector(`#${id}-problem`)?.cloneNode(true);
  if (!(problem instanceof HTMLElement)) return '';
  for (const icon of problem.querySelectorAll('[aria-hidden]')) icon.remove();
  return problem.textContent.trim();
}
