import { expect, it } from 'vitest';
import { blankTabOpening, systemBrowserOpening } from './browser';

function tab(): Window {
  return { opener: {}, closed: false, location: { href: 'about:blank' }, close: () => undefined } as unknown as Window;
}

it('PRJ-041: the web opens a blank tab at once, cuts its opener, and sets its location later', async () => {
  const opened = tab();
  const opening = blankTabOpening(() => opened);
  expect(opened.opener).toBeNull();
  expect(opened.location.href).toBe('about:blank');
  expect(await opening.open('https://hub.example/connect/github?ticket=T')).toBe(true);
  expect(opened.location.href).toBe('https://hub.example/connect/github?ticket=T');
});

it('PRJ-041: a blocked or closed tab reports that the link was not shown', async () => {
  expect(await blankTabOpening(() => null).open('https://hub.example/x')).toBe(false);
  const closed = tab();
  Object.defineProperty(closed, 'closed', { value: true });
  expect(await blankTabOpening(() => closed).open('https://hub.example/x')).toBe(false);
});

it('PRJ-041: the desktop gives the link to the system browser command, and a refusal is not shown', async () => {
  const sent: string[] = [];
  expect(await systemBrowserOpening((url) => Promise.resolve(void sent.push(url))).open('https://hub.example/x')).toBe(true);
  expect(sent).toEqual(['https://hub.example/x']);
  expect(await systemBrowserOpening(() => Promise.reject(new Error('no'))).open('https://hub.example/x')).toBe(false);
});
