import type { BrowserContext, Page } from '@playwright/test';
import { expect, test } from './test-hub';
import { HUB_ORIGIN, OPERATOR, SESSION_COOKIE } from './synthetic/access';
import type { RecordedRequest } from './synthetic/recorder';

// Browser sign-in and sign-out (WP-44 E-4): the page sends signInBrowser and never login, no token or password is ever in
// reach of script, and each change of the cookie loads a new page, so no socket keeps the old cookie (CLI-003, D-3).

const WRONG_PASSWORD = 'wrong-horse-synthetic';
const OLD_PAGE_MARK = 'animaOldPage';

/** Marks the page now shown; a full load starts a new one without the mark, a router move keeps it. */
async function markPage(page: Page): Promise<void> {
  await page.evaluate((mark) => Reflect.set(window, mark, true), OLD_PAGE_MARK);
}

async function isOldPage(page: Page): Promise<boolean> {
  return page.evaluate((mark) => mark in window, OLD_PAGE_MARK);
}

/** Every console line of the page, from now on and across loads. */
function consoleLines(page: Page): string[] {
  const lines: string[] = [];
  page.on('console', (message) => lines.push(message.text()));
  return lines;
}

/** What script on the page can read: `document.cookie`, both web storages and the whole DOM, with field values. */
function scriptReach(page: Page): Promise<string> {
  return page.evaluate(() => {
    const values = [...document.querySelectorAll('input')].map((field) => field.value);
    return JSON.stringify([document.cookie, { ...localStorage }, { ...sessionStorage }, document.documentElement.outerHTML, values]);
  });
}

async function sessionCookie(context: BrowserContext): Promise<string | undefined> {
  return (await context.cookies(HUB_ORIGIN)).find((found) => found.name === SESSION_COOKIE)?.value;
}

async function signInWith(page: Page, password: string): Promise<void> {
  await page.getByLabel('User name', { exact: true }).fill(OPERATOR.username);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Code from your authenticator', { exact: true }).fill(OPERATOR.code);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}

/** Opens the sign-in page and waits until its form works; the `load` event (fonts, icons) is not what the specs need. */
async function openSignIn(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/sign-in`, { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeEnabled();
}

/** Whether `requests` hold a graphql-ws socket the hub signed in by the cookie alone, with no Authorization in its payload. */
function cookieSocketAcknowledged(requests: readonly RecordedRequest[]): boolean {
  const opened = requests.filter((request) => request.channel === 'graphql-socket' && request.operation === 'connection_init');
  return opened.some((socket) => socket.signedIn && socket.hadCookie && !socket.hadAuthorization);
}

test.describe('signed out, a good sign-in', () => {
  test('D-3, E-4: sign-in sends signInBrowser, never login, and loads /devices as a new page whose socket signs in by cookie', async ({
    page,
    hub,
    watch,
  }) => {
    await openSignIn(page);
    expect(cookieSocketAcknowledged(hub.recording.requests)).toBe(false);
    await markPage(page);
    await signInWith(page, OPERATOR.password);
    // At commit the old page is gone and the new one has run no script yet: later socket records are the new page's.
    await page.waitForURL(`${HUB_ORIGIN}/devices`, { waitUntil: 'commit' });
    const loadedAt = hub.recording.requests.length;
    await expect(page.getByText('win-desktop', { exact: true }).first()).toBeVisible();

    expect(await isOldPage(page)).toBe(false);
    const signIns = hub.recording.where({ operation: 'SignInBrowser' });
    expect(signIns.map(({ variables, hadAuthorization, origin }) => ({ variables, hadAuthorization, origin }))).toEqual([
      { variables: { input: { username: 'ann', password: OPERATOR.password, totpCode: OPERATOR.code } }, hadAuthorization: false, origin: HUB_ORIGIN },
    ]);
    expect(hub.recording.where({ operation: 'Login' })).toEqual([]);
    expect(hub.recording.requests.every((request) => !request.hadAuthorization)).toBe(true);
    await expect.poll(() => cookieSocketAcknowledged(hub.recording.requests.slice(loadedAt))).toBe(true);
    expect(hub.recording.where({ operation: 'Devices' }).every((request) => request.signedIn)).toBe(true);
    // The form submits by script under the K7 `form-action 'none'`; nothing in the flow breaks the policy.
    expect(watch.cspViolations).toEqual([]);
  });

  test('CLI-003, LOG-007: after sign-in no token or password is in the cookie script reads, web storage, the DOM or the console', async ({
    page,
    context,
  }) => {
    const lines = consoleLines(page);
    await openSignIn(page);
    await signInWith(page, OPERATOR.password);
    await page.waitForURL(`${HUB_ORIGIN}/devices`);
    await expect(page.getByText('win-desktop', { exact: true }).first()).toBeVisible();

    const token = await sessionCookie(context);
    expect(token).toBeDefined();
    expect(await page.evaluate(() => document.cookie)).toBe('');
    const reach = await scriptReach(page);
    for (const secret of [token ?? 'missing', OPERATOR.password]) {
      expect(reach).not.toContain(secret);
      expect(lines.filter((line) => line.includes(secret))).toEqual([]);
    }
  });

});

test.describe('signed out, refusals', () => {
  test('LOG-007: a wrong password shows the fixed message, sets no cookie and leaves the password nowhere on the page', async ({
    page,
    context,
    hub,
  }) => {
    const lines = consoleLines(page);
    await openSignIn(page);
    await signInWith(page, WRONG_PASSWORD);
    await expect(page.getByRole('alert')).toHaveText('The user name, password or code is wrong.');

    expect(page.url()).toBe(`${HUB_ORIGIN}/sign-in`);
    expect(await sessionCookie(context)).toBeUndefined();
    expect(hub.recording.where({ operation: 'SignInBrowser' })).toHaveLength(1);
    expect(await scriptReach(page)).not.toContain(WRONG_PASSWORD);
    expect(lines.filter((line) => line.includes(WRONG_PASSWORD))).toEqual([]);
  });

  test('LOG-007: FORBIDDEN shows the browser wording and never the hub text', async ({ page }) => {
    // The page route answers before the synthetic hub, as a hub that refuses this network or this page's Origin would.
    await page.route('**/graphql', async (route) => {
      if (!(route.request().postData() ?? '').includes('signInBrowser')) return route.fallback();
      const errors = [{ message: 'hub text: refused for 203.0.113.9', extensions: { code: 'FORBIDDEN' } }];
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ data: null, errors }) });
    });
    await openSignIn(page);
    await signInWith(page, OPERATOR.password);
    await expect(page.getByRole('alert')).toHaveText('The hub refused sign-in from here. Check the VPN client and the page address.');
    expect(await page.content()).not.toContain('203.0.113.9');
  });

  test('CLI-002: the sign-in page has no hub address field and no way to save one', async ({ page }) => {
    await openSignIn(page);
    await expect(page.getByLabel('Hub address')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Save address' })).toHaveCount(0);
    await expect(page.locator('main form')).toHaveCount(1);
  });

  test('API-003: a signed-out visit to /devices goes to /sign-in', async ({ page, hub }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.waitForURL(`${HUB_ORIGIN}/sign-in`);
    await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible();
    expect(hub.recording.where({ operation: 'Me' })[0]?.signedIn).toBe(false);
  });
});

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('E-4: Sign out sends logout, the hub clears the cookie, and /sign-in loads as a new page that stays signed out', async ({
    page,
    context,
    hub,
    watch,
  }) => {
    await page.goto(`${HUB_ORIGIN}/settings`);
    await expect(page.getByText('ann', { exact: true })).toBeVisible();
    await markPage(page);
    await page.getByRole('button', { name: 'Sign out', exact: true }).click();
    await page.waitForURL(`${HUB_ORIGIN}/sign-in`);
    await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible();

    expect(await isOldPage(page)).toBe(false);
    expect(hub.recording.where({ operation: 'Logout' }).map(({ hadCookie, signedIn }) => ({ hadCookie, signedIn }))).toEqual([
      { hadCookie: true, signedIn: true },
    ]);
    expect(await sessionCookie(context)).toBeUndefined();
    expect(watch.cspViolations).toEqual([]);
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.waitForURL(`${HUB_ORIGIN}/sign-in`);
  });
});
