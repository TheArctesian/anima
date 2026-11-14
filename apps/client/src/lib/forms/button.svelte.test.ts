// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import FormButton from './FormButton.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const text = createRawSnippet(() => ({ render: () => '<span>Save</span>' }));

function button(props: Record<string, unknown> = {}): HTMLButtonElement {
  app = mount(FormButton, { target: document.body, props: { children: text, ...props } });
  flushSync();
  const found = document.querySelector('button');
  if (!found) throw new Error('no button');
  return found;
}

it('UI-029: a button is secondary and md by default, and never submits unless told to', () => {
  const found = button();
  expect(found.classList.contains('secondary')).toBe(true);
  expect(found.classList.contains('md')).toBe(true);
  expect(found.type).toBe('button');
});

it('UI-029: the tone and size props select the matching role classes; fill applies to danger', () => {
  const found = button({ tone: 'danger', size: 'sm', fill: true });
  expect([...found.classList].filter((name) => !name.startsWith('svelte-'))).toEqual(['danger', 'sm', 'fill']);
});

it('UI-015: an icon-only button is named and has a tooltip from its label, and shows only the icon', () => {
  const found = button({ children: undefined, icon: 'carbon:trash-can', label: 'Delete key' });
  expect(found.getAttribute('aria-label')).toBe('Delete key');
  expect(found.title).toBe('Delete key');
  expect(found.classList.contains('icon-only')).toBe(true);
  expect(found.querySelector('.label, span:not(.icon)')).toBeNull();
});

it('UI-026: a leading icon is hidden from assistive technology and sits before the label', () => {
  const found = button({ icon: 'carbon:add' });
  expect(found.firstElementChild?.getAttribute('aria-hidden')).toBe('true');
  expect(found.textContent).toContain('Save');
});

it('UI-013: a busy button is aria-busy and aria-disabled, keeps focus, and ignores clicks', () => {
  const onclick = vi.fn();
  const found = button({ busy: true, onclick });
  expect(found.getAttribute('aria-busy')).toBe('true');
  expect(found.getAttribute('aria-disabled')).toBe('true');
  expect(found.disabled).toBe(false);
  found.click();
  expect(onclick).not.toHaveBeenCalled();
});

it('UI-013: a caller-set aria-disabled blocks the click as well; a native disabled button is disabled', () => {
  const onclick = vi.fn();
  button({ 'aria-disabled': true, onclick }).click();
  expect(onclick).not.toHaveBeenCalled();
  void unmount(app ?? {});
  document.body.replaceChildren();
  expect(button({ disabled: true }).disabled).toBe(true);
});

it('an idle button passes the click on', () => {
  const onclick = vi.fn();
  button({ onclick }).click();
  expect(onclick).toHaveBeenCalledOnce();
});

it('MOT-008: with feedback, the three labels share one cell and only the current one is exposed', () => {
  const found = button({ feedback: 'pending', pendingLabel: 'Saving', doneLabel: 'Saved' });
  const labels = [...found.querySelectorAll('.label')];
  expect(labels.map((label) => label.getAttribute('aria-hidden'))).toEqual(['true', 'false', 'true']);
  expect(found.getAttribute('aria-busy')).toBe('true');
});
