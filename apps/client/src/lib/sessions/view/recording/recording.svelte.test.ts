// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import RecordingBadge, { type RecordingState } from './RecordingBadge.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function show(initial: RecordingState): { set: (next: RecordingState) => void } {
  const props = $state({ recording: initial });
  app = mount(RecordingBadge, { target: document.body, props });
  flushSync();
  return {
    set: (next) => {
      props.recording = next;
      flushSync();
    },
  };
}

const live = () => document.querySelector('[role="status"]');

it('RA-040: shows nothing while recording is off', () => {
  show('OFF');
  expect(live()?.textContent.trim()).toBe('');
  expect(live()?.children).toHaveLength(0);
});

it('RA-040: shows "Recording" in a status element while recording is on', () => {
  show('ON');
  expect(live()?.textContent.trim()).toBe('Recording');
});

it('D-14: shows "Recording stopped" when the hub stops the recording', () => {
  show('STOPPED');
  expect(live()?.textContent.trim()).toBe('Recording stopped');
});

it('D-14: a change from ON to STOPPED updates the same status element, so it is announced', () => {
  const badge = show('ON');
  const before = live();
  expect(before?.textContent.trim()).toBe('Recording');
  badge.set('STOPPED');
  expect(live()).toBe(before);
  expect(live()?.textContent.trim()).toBe('Recording stopped');
});

it('RA-040: a change from OFF to ON fills the status element that was already present', () => {
  const badge = show('OFF');
  const before = live();
  expect(before?.textContent.trim()).toBe('');
  badge.set('ON');
  expect(live()).toBe(before);
  expect(live()?.textContent.trim()).toBe('Recording');
});
