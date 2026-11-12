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

/** The six code boxes of step 2, in order. */
export function boxes(target: HTMLElement): HTMLInputElement[] {
  return [...target.querySelectorAll<HTMLInputElement>('[role="group"] input')];
}

export async function settle(): Promise<void> {
  await vi.waitFor(() => {
    flushSync();
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** Step 1's Continue, with the two fields filled. */
export async function continueWith(target: HTMLElement, username: string, password: string): Promise<void> {
  type(target, 'username', username);
  type(target, 'password', password);
  await submit(target, 0);
}

/** A key pressed in box `index`; returns the event so a test can see whether the page took it. */
export function press(target: HTMLElement, index: number, key: string): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
  boxes(target)[index]?.dispatchEvent(event);
  flushSync();
  return event;
}

/** `text` pasted into box `index`. */
export async function pasteInto(target: HTMLElement, index: number, text: string): Promise<void> {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', { value: { getData: () => text } });
  boxes(target)[index]?.dispatchEvent(event);
  await settle();
}

/** A whole code typed digit by digit, as the operator would. */
export async function typeCode(target: HTMLElement, code: string): Promise<void> {
  [...code].forEach((digit, index) => press(target, index, digit));
  await settle();
}
