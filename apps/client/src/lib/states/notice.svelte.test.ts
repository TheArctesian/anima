// @vitest-environment jsdom
import { mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Notice from './Notice.svelte';
import { noticeRole, type NoticeTone } from './notice';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it.each<[NoticeTone, string]>([
  ['info', 'status'],
  ['success', 'status'],
  ['warning', 'status'],
  ['error', 'alert'],
])('SET-005: a %s notice has the role %s', (tone, role) => {
  expect(noticeRole(tone)).toBe(role);
  app = mount(Notice, { target: document.body, props: { tone, title: 'Result', line: 'What it means.' } });
  const notice = document.querySelector(`[role="${role}"]`);
  expect(notice?.querySelector('.title')?.textContent).toBe('Result');
  expect(notice?.querySelector('.line')?.textContent).toBe('What it means.');
  expect(notice?.querySelector('.icon')).not.toBeNull();
});

it('SET-005: the detail line is optional', () => {
  app = mount(Notice, { target: document.body, props: { tone: 'success', title: 'Saved' } });
  expect(document.querySelector('.line')).toBeNull();
});
