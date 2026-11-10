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
  const props = $state({ state: initial });
  app = mount(RecordingBadge, { target: document.body, props });
  flushSync();
  return {
    set: (next) => {
      props.state = next;
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
  badge.set('STOPPED');
  expect(live()).toBe(before);
  expect(live()?.textContent.trim()).toBe('Recording stopped');
});

it('MOT-011, MAT-008: the badge is static, with no animation or blur', () => {
  show('ON');
  const html = document.body.innerHTML;
  expect(html).not.toMatch(/pulse|blink|animate|blur/i);
});
