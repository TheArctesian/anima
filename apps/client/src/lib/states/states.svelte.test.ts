// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import EmptyState from './EmptyState.svelte';
import ErrorState from './ErrorState.svelte';
import LoadingState from './LoadingState.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const text = (html: string) => createRawSnippet(() => ({ render: () => html }));

it('UI-030 and DEV-005: the empty state has a short title, one line and one action', () => {
  app = mount(EmptyState, { target: document.body, props: { title: 'No devices yet', line: 'Add your first device.', action: text('<button>Add device</button>') } });
  expect(document.querySelector('h3')?.textContent).toBe('No devices yet');
  expect(document.querySelector('p')?.textContent).toBe('Add your first device.');
  expect(document.querySelectorAll('button')).toHaveLength(1);
});

it('UI-030: the error state names what failed, offers "Try again" and keeps the cause in its slot', () => {
  const onRetry = vi.fn();
  app = mount(ErrorState, { target: document.body, props: { message: 'Devices did not load.', onRetry, cause: text('<span class="cause">503</span>') } });
  flushSync();
  expect(document.querySelector('[role="alert"] p')?.textContent).toBe('Devices did not load.');
  const retry = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Try again');
  retry?.click();
  expect(onRetry).toHaveBeenCalledOnce();
  expect(document.querySelector('.next .cause')?.textContent).toBe('503');
  expect(document.querySelector('[role="alert"] p')?.textContent).not.toContain('503');
});

it('UI-030: the loading state is a labelled busy status with the requested rows', () => {
  app = mount(LoadingState, { target: document.body, props: { label: 'Loading devices', rows: 4 } });
  const status = document.querySelector('[role="status"]');
  expect(status?.getAttribute('aria-label')).toBe('Loading devices');
  expect(status?.getAttribute('aria-busy')).toBe('true');
  expect(status?.querySelectorAll('.row')).toHaveLength(4);
});
