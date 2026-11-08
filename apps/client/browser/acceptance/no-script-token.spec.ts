import type { Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { consoleLines, openSignIn, scriptReach, sessionCookie, signInWith } from '../sign-in-steps';
import { HUB_ORIGIN, OPERATOR } from '../synthetic/access';

// Plan criterion 1, client side: the operator signs in with password and TOTP, and afterwards the browser holds only an
// HttpOnly cookie, with no token in reach of script. `sign-in.spec.ts` covers the cookie string, both web storages, the DOM and
// the console after a good sign-in; these add the authenticator code and the stores that spec does not list.

const WRONG_CODE = '000000';

/** Names of every store a page can keep data in, beyond cookies and the two web storages. */
function otherStores(page: Page): Promise<{ databases: string[]; caches: string[]; workers: number }> {
  return page.evaluate(async () => ({
    databases: (await indexedDB.databases()).map((database) => database.name ?? ''),
    caches: await caches.keys(),
    workers: (await navigator.serviceWorker.getRegistrations()).length,
  }));
}

test.describe('signed out', () => {
  test('CLI-001, SEC-002: the authenticator code is part of the sign-in: a wrong code is refused with no cookie, the right one signs in', async ({
    page,
    context,
    hub,
  }) => {
    await openSignIn(page);
    await signInWith(page, OPERATOR.password, WRONG_CODE);
    await expect(page.getByRole('alert')).toHaveText('The user name, password or code is wrong.');
    expect(await sessionCookie(context)).toBeUndefined();

    await signInWith(page, OPERATOR.password);
    await page.waitForURL(`${HUB_ORIGIN}/devices`);
    expect(await sessionCookie(context)).toBeDefined();
    expect(hub.recording.where({ operation: 'SignInBrowser' }).map((sent) => (sent.variables as { input: { totpCode: string } }).input.totpCode)).toEqual([
      WRONG_CODE,
      OPERATOR.code,
    ]);
  });

  test('CLI-003, SEC-002, D-3: after a password and TOTP sign-in, the cookie is HttpOnly and no token is in any store script can read, the URL or the console', async ({
    page,
    context,
  }) => {
    const lines = consoleLines(page);
    await openSignIn(page);
    await signInWith(page, OPERATOR.password);
    await page.waitForURL(`${HUB_ORIGIN}/devices`);
    await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();

    const token = await sessionCookie(context);
    expect(token).toBeDefined();
    const [cookie] = await context.cookies(HUB_ORIGIN);
    expect(cookie?.httpOnly).toBe(true);

    // A reload starts a new page: the secrets must be absent from it too, not only from the page that took the password.
    for (const stage of ['after sign-in', 'after a reload']) {
      if (stage === 'after a reload') await page.reload({ waitUntil: 'domcontentloaded' });
      await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
      const reach = JSON.stringify([await scriptReach(page), await otherStores(page), page.url(), await page.evaluate(() => [window.name, JSON.stringify(history.state)])]);
      for (const secret of [token ?? 'missing', OPERATOR.password]) expect(reach, `${stage}: ${secret === token ? 'token' : 'password'}`).not.toContain(secret);
      expect(await otherStores(page)).toEqual({ databases: [], caches: [], workers: 0 });
    }
    for (const secret of [token ?? 'missing', OPERATOR.password]) expect(lines.filter((line) => line.includes(secret))).toEqual([]);
  });
});
