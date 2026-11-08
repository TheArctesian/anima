import type { BrowserContext, Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';
import { DEVICE_IDS } from './screens';

// Back and Forward across the `/shared` line (WP-44 E-3, T1 `holdsAt`): `/shared` and below it is a bare document with no
// operator hub, socket or shell, and every other route is the shell. A navigation across that line is a full load, so
// Back from one to the other brings the right kind of document, never a stale one of the other kind (SEC-002, D-7).
// `/shared` is WP-49's; until it exists the line is tested at its 404 page, which the root layout renders bare too.

const STARTED_AT = 'animaStartedAt';
const OLD_PAGE_MARK = 'animaOldPage';

/** Every document notes where it started and every WebSocket it opens, in `sessionStorage`, which a full load keeps. */
async function trackDocuments(context: BrowserContext): Promise<void> {
  await context.addInitScript(
    ([started]) => {
      if (window.top !== window) return;
      Reflect.set(window, started ?? '', location.pathname + location.hash);
      const Native = window.WebSocket;
      window.WebSocket = class extends Native {
        constructor(...args: ConstructorParameters<typeof WebSocket>) {
          super(...args);
          const sockets = JSON.parse(sessionStorage.getItem('sockets') ?? '[]') as string[];
          sessionStorage.setItem('sockets', JSON.stringify([...sockets, location.pathname + location.hash]));
        }
      };
    },
    [STARTED_AT] as const,
  );
}

/** What the document on screen is: where it started, whether it is the shell, whether it is the one the test marked. */
async function documentOnScreen(page: Page): Promise<{ startedAt: string; shell: boolean; marked: boolean; sockets: string[] }> {
  return page.evaluate(
    ([started, mark]) => ({
      startedAt: String(Reflect.get(window, started ?? '')),
      // Only the shell has the navigation bar.
      shell: document.querySelector('nav') !== null,
      marked: Reflect.has(window, mark ?? ''),
      sockets: JSON.parse(sessionStorage.getItem('sockets') ?? '[]') as string[],
    }),
    [STARTED_AT, OLD_PAGE_MARK] as const,
  );
}

/** Follows a link as a click on an anchor does, so the client's router sees it (a `goto` would be a new document at once). */
async function followLink(page: Page, href: string): Promise<void> {
  await page.evaluate((target) => {
    const anchor = document.createElement('a');
    anchor.href = target;
    anchor.textContent = 'follow';
    document.body.append(anchor);
    anchor.click();
  }, href);
}

test.use({ signedIn: true });
test.beforeEach(async ({ context }) => {
  await trackDocuments(context);
});

test('SEC-002, D-7: from the shell to /shared and Back, the shell comes back as a fresh document, not the bare page', async ({ page, hub }) => {
  await page.goto(`${HUB_ORIGIN}/devices`, { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
  await page.evaluate((mark) => Reflect.set(window, mark, true), OLD_PAGE_MARK);

  await followLink(page, '/shared#tok');
  await page.waitForURL(`${HUB_ORIGIN}/shared#tok`);
  await expect.poll(() => documentOnScreen(page).then((found) => found.marked)).toBe(false);
  expect(await documentOnScreen(page)).toMatchObject({ startedAt: '/shared#tok', shell: false, marked: false });
  const requestsAtBare = hub.recording.requests.length;

  await page.goBack();
  await page.waitForURL(`${HUB_ORIGIN}/devices`);
  await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
  // The document that shows /devices began at /devices: it is the shell started afresh, not the bare document wearing a new address.
  expect(await documentOnScreen(page)).toMatchObject({ startedAt: '/devices', shell: true });
  expect(hub.recording.requests.length).toBeGreaterThan(requestsAtBare);
  // No bare document ever opened a socket; every socket belongs to a shell document.
  expect((await documentOnScreen(page)).sockets.filter((started) => started.startsWith('/shared'))).toEqual([]);
});

test('SEC-002, D-7: from /shared to an operator page and Back, the bare page comes back as a fresh document with no hub', async ({ page, hub }) => {
  await page.goto(`${HUB_ORIGIN}/shared#tok`, { waitUntil: 'domcontentloaded' });
  expect(await documentOnScreen(page)).toMatchObject({ startedAt: '/shared#tok', shell: false });
  await page.evaluate((mark) => Reflect.set(window, mark, true), OLD_PAGE_MARK);
  // A bare page calls no operator operation: the hub has heard nothing yet.
  expect(hub.recording.requests).toEqual([]);

  await followLink(page, `/devices/${DEVICE_IDS.nas}`);
  await page.waitForURL(`${HUB_ORIGIN}/devices/${DEVICE_IDS.nas}`);
  await expect(page.getByRole('heading', { level: 1, name: 'nas' })).toBeVisible();
  expect(await documentOnScreen(page)).toMatchObject({ startedAt: `/devices/${DEVICE_IDS.nas}`, shell: true, marked: false });
  const requestsAtShell = hub.recording.requests.length;
  expect(requestsAtShell).toBeGreaterThan(0);

  await page.goBack();
  await page.waitForURL(`${HUB_ORIGIN}/shared#tok`);
  await expect.poll(() => documentOnScreen(page).then((found) => found.shell)).toBe(false);
  expect(await documentOnScreen(page)).toMatchObject({ startedAt: '/shared#tok', shell: false });
  await expect(page.getByRole('link', { name: 'Settings' })).toHaveCount(0);
  // The bare document started no hub: nothing was asked of the hub since the shell ended, and its sockets all came from shell documents.
  expect(hub.recording.requests.length).toBe(requestsAtShell);
  expect((await documentOnScreen(page)).sockets.filter((started) => started.startsWith('/shared'))).toEqual([]);
});
