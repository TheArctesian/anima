import { expect, type BrowserContext, type Page } from '@playwright/test';
import { HUB_ORIGIN, OPERATOR, SESSION_COOKIE } from './synthetic/access';

// The steps of a browser sign-in and the places a script on the page could read a secret from. Shared by the sign-in specs
// and the acceptance specs, so one definition says what "no token or password in reach of script" means (CLI-003, D-3).

/** Every console line of the page, from now on and across loads. */
export function consoleLines(page: Page): string[] {
  const lines: string[] = [];
  page.on('console', (message) => lines.push(message.text()));
  return lines;
}

/** What script on the page can read: `document.cookie`, both web storages and the whole DOM, with field values. */
export function scriptReach(page: Page): Promise<string> {
  return page.evaluate(() => {
    const values = [...document.querySelectorAll('input')].map((field) => field.value);
    return JSON.stringify([document.cookie, { ...localStorage }, { ...sessionStorage }, document.documentElement.outerHTML, values]);
  });
}

export async function sessionCookie(context: BrowserContext): Promise<string | undefined> {
  return (await context.cookies(HUB_ORIGIN)).find((found) => found.name === SESSION_COOKIE)?.value;
}

/** Fills the sign-in form and submits it; `code` is the authenticator code, the good one unless a spec says otherwise. */
export async function signInWith(page: Page, password: string, code: string = OPERATOR.code): Promise<void> {
  await page.getByLabel('User name', { exact: true }).fill(OPERATOR.username);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Code from your authenticator', { exact: true }).fill(code);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}

/** Opens the sign-in page and waits until its form works; the `load` event (fonts, icons) is not what the specs need. */
export async function openSignIn(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/sign-in`, { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeEnabled();
}
