// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import CheckEditor from './CheckEditor.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

it('MOT-007: Add check is a fixed-size dialog with Cancel and Add check in a footer that does not scroll', async () => {
  const onSave = vi.fn(() => Promise.resolve({ ok: true as const, value: undefined }));
  app = mount(CheckEditor, { target: document.body, props: { open: true, check: null, checks: [], onSave, onClose: () => undefined } });
  flushSync();

  const dialog = document.querySelector('[role="dialog"]');
  const footer = dialog?.querySelector(':scope > .body + .footer');
  expect(dialog?.classList.contains('form')).toBe(true);
  expect(footer?.contains(button('Cancel'))).toBe(true);
  expect(footer?.contains(button('Add check'))).toBe(true);
  expect(button('Add check').form).toBe(dialog?.querySelector(':scope > .body form'));

  const name = [...document.querySelectorAll('label')].find((label) => label.textContent.trim() === 'Name')?.control as HTMLInputElement;
  name.value = '';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  button('Add check').click();
  await tick();
  expect(name.getAttribute('aria-invalid')).toBe('true');
  expect(onSave).not.toHaveBeenCalled();
});
