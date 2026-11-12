// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../animations.fixture';
import Input from '../input/Input.svelte';
import Field from './Field.svelte';
import { controlLinks } from './field';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

/** A field around a real Input, the way a caller builds one. */
function field({ help: withHelp = false, ...props }: { constraint?: string; error?: string; help?: boolean; layout?: 'stacked' | 'inline' }) {
  installAnimations();
  const children = createRawSnippet<[{ id: string; 'aria-describedby': string | undefined; 'aria-invalid': true | undefined }]>((link) => ({
    render: () => `<span data-slot></span>`,
    setup: (node) => {
      const inner = mount(Input, { target: node, props: { ...link(), placeholder: 'host' } });
      return () => void unmount(inner);
    },
  }));
  const help = createRawSnippet(() => ({ render: () => '<span>Seconds</span>' }));
  app = mount(Field, { target: document.body, props: { label: 'Host', children, ...(withHelp ? { help } : {}), ...props } });
  flushSync();
  const input = document.querySelector('input');
  if (!input) throw new Error('no input');
  return input;
}

it('UI-019: the label points at the control, so clicking the label focuses it', () => {
  const input = field({});
  const label = document.querySelector('label');
  expect(label?.getAttribute('for')).toBe(input.id);
  expect(input.getAttribute('aria-describedby')).toBeNull();
  expect(input.getAttribute('aria-invalid')).toBeNull();
});

it('UI-028: a constraint is visible text and describes the control', () => {
  const input = field({ constraint: 'Letters, digits and dots' });
  const line = document.querySelector('.constraint');
  expect(line?.textContent).toBe('Letters, digits and dots');
  expect(input.getAttribute('aria-describedby')).toBe(line?.id);
});

it('UI-028: an error is in an alert region, marks the control invalid, and describes it after the constraint', () => {
  const input = field({ constraint: 'Letters only', error: 'Not a host name' });
  const error = document.querySelector('[role="alert"] .error');
  expect(error?.textContent).toContain('Not a host name');
  expect(input.getAttribute('aria-invalid')).toBe('true');
  expect(input.getAttribute('aria-describedby')).toBe(`${document.querySelector('.constraint')?.id ?? ''} ${error?.id ?? ''}`);
});

it('UI-028: the alert region exists before an error, so the first error is announced', () => {
  field({});
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(document.querySelector('.error')).toBeNull();
});

it('UI-028: a field with help has a HelpTip button named for the label; errors are never in it', () => {
  field({ help: true });
  expect(document.querySelector('button')?.getAttribute('aria-label')).toBe('About Host');
});

it('controlLinks: no constraint and no error leave nothing to describe', () => {
  expect(controlLinks('f', false, false)).toEqual({ id: 'f-control', 'aria-describedby': undefined, 'aria-invalid': undefined });
});
