// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import SplitButton from './SplitButton.svelte';
import type { SplitChoice } from './split';

endTransitionsAtOnce();

const CHOICES: readonly SplitChoice[] = [
  { id: 'ssh', label: 'SSH' },
  { id: 'vnc', label: 'VNC' },
  { id: 'wake', label: 'Wake' },
];

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show() {
  const onMain = vi.fn();
  const onChoose = vi.fn();
  app = mount(SplitButton, {
    target: document.body,
    props: { label: 'Connect', choices: CHOICES, menuLabel: 'More ways to connect to nas', onMain, onChoose },
  });
  const chevron = document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
  if (chevron === null) throw new Error('no chevron');
  return { onMain, onChoose, chevron };
}

const key = (target: Element, name: string) => target.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true }));
const focused = () => document.activeElement?.textContent.trim();

it('UI-015, UR-7: the main button does the usual action and the chevron is named for what it opens', () => {
  const { onMain, onChoose, chevron } = show();
  document.querySelector<HTMLButtonElement>('button')?.click();
  expect(onMain).toHaveBeenCalledOnce();
  expect(onChoose).not.toHaveBeenCalled();
  expect(chevron.getAttribute('aria-label')).toBe('More ways to connect to nas');
  expect(chevron.getAttribute('aria-expanded')).toBe('false');
});

it('WCAG 2.1.1: the arrow keys open the menu and move through it, Escape gives focus back to the chevron', async () => {
  const { chevron } = show();
  chevron.focus();
  key(chevron, 'ArrowDown');
  await tick();
  flushSync();
  expect(chevron.getAttribute('aria-expanded')).toBe('true');
  expect(focused()).toBe('SSH');
  const menu = document.querySelector('[role="menu"]');
  if (menu === null) throw new Error('no menu');
  key(menu, 'ArrowUp');
  expect(focused()).toBe('Wake');
  key(menu, 'Escape');
  await vi.waitFor(() => {
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });
  expect(document.activeElement).toBe(chevron);
});

it('UR-7: choosing an item runs it and closes the menu', async () => {
  const { onChoose, chevron } = show();
  chevron.click();
  await tick();
  flushSync();
  [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')][1]?.click();
  expect(onChoose).toHaveBeenCalledWith(CHOICES[1], chevron);
  await vi.waitFor(() => {
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });
});
