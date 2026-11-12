// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../../animations.fixture';
import Modal from '../Modal.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
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

const html = (markup: string) => createRawSnippet(() => ({ render: () => markup }));

function open(props: Record<string, unknown> = {}): { onClose: () => void; opener: HTMLButtonElement } {
  const opener = document.createElement('button');
  document.body.append(opener);
  opener.focus();
  const onClose = vi.fn();
  app = mount(Modal, {
    target: document.body,
    props: { open: true, title: 'Add device', onClose, children: html('<input aria-label="Name" />'), ...props },
  });
  flushSync();
  return { onClose, opener };
}

it.each([
  ['sm', '6'],
  ['md', '8'],
  ['lg', '12'],
])('MAT-003: the %s preset sets the surface width to %s steps', (preset, steps) => {
  open({ preset });
  expect(document.querySelector<HTMLElement>('[role="dialog"]')?.style.getPropertyValue('--modal-width')).toBe(steps);
});

it('MAT-003: the footer snippet sits below the body, outside the scrolling area, with the scrim behind it', () => {
  open({ preset: 'md', footer: html('<span><button>Cancel</button><button>Add</button></span>') });
  const surface = document.querySelector('[role="dialog"]');
  expect([...(surface?.querySelectorAll(':scope > .footer button') ?? [])].map((button) => button.textContent)).toEqual(['Cancel', 'Add']);
  expect(surface?.querySelector(':scope > .body button')).toBeNull();
  expect(document.querySelector('.scrim')).not.toBeNull();
});

it('Dialog: Escape closes it and a dismissible dialog has a Close button in its header', () => {
  const { onClose } = open({ dismissible: true });
  expect(document.querySelector('[role="dialog"] .close')?.getAttribute('aria-label')).toBe('Close');
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  expect(onClose).toHaveBeenCalledOnce();
});

it('Dialog: focus starts inside, Tab stays inside, and focus returns to the opener on close', () => {
  const state = $state({ open: true });
  const opener = document.createElement('button');
  document.body.append(opener);
  opener.focus();
  app = mount(Modal, {
    target: document.body,
    props: {
      get open() {
        return state.open;
      },
      title: 'Add device',
      onClose: () => undefined,
      footer: html('<span><button>Cancel</button><button>Add</button></span>'),
      children: html('<input aria-label="Name" />'),
    },
  });
  flushSync();
  const surface = document.querySelector<HTMLElement>('[role="dialog"]');
  expect(surface?.contains(document.activeElement)).toBe(true);
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
  document.querySelector<HTMLElement>('.footer button:last-child')?.focus();
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
  expect(surface?.contains(document.activeElement)).toBe(true);
  state.open = false;
  flushSync();
  expect(document.activeElement).toBe(opener);
});
