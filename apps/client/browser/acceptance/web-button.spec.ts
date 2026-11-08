import type { BrowserContext, Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { openDevices } from './screens';

// Plan criterion 3 (ruled in R-087): in the web client the Web button opens a new browser tab straight to the device, with
// `noopener` and `noreferrer`, for `http:` and `https:` addresses only. When an interface has a pinned certificate the button
// opens a menu, and each choice shows its own pin, tied to it by `aria-describedby` (WEB-011, D-5). `lib/web-interfaces/open`
// has jsdom tests; these run the real engines, where the tab, the opener and the referrer are the browser's own.

const MAC_MINI_PAGE = 'http://mac-mini.annex.example:3000/status';
const NAS_ADMIN_PAGE = 'https://nas.annex.example:5001/';
// The fixture's SHA-256 as the client shows it: upper case, in pairs.
const NAS_ADMIN_PIN = '5C:0E:8B:4D:1F:7A:2C:93:3F:2A:9C:41:D0:7B:58:E6:A1:C4:E9:B2:70:5D:38:F6:C8:A4:B1:E0:7D:92:F3:5A';
const WINDOW_OPEN = 'animaWindowOpenCalls';

interface DevicePages {
  /** The `Referer` each device page request carried: WebKit sends the header with an empty value, the others leave it out. */
  readonly referers: string[];
}

/** Answers the device addresses the fixtures name, so a tab opened to one shows a page; the hub aborts every foreign origin. */
async function serveDevicePages(context: BrowserContext): Promise<DevicePages> {
  const referers: string[] = [];
  await context.route(/\.annex\.example/, async (route) => {
    referers.push((await route.request().allHeaders()).referer ?? '');
    await route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><title>device</title><p>device page</p>' });
  });
  return { referers };
}

/** Records each `window.open` the page makes, with its arguments, and still makes it. */
async function watchWindowOpen(context: BrowserContext): Promise<void> {
  await context.addInitScript((name) => {
    const calls: unknown[][] = [];
    Reflect.set(window, name, calls);
    const open = window.open.bind(window);
    window.open = (...args: Parameters<typeof window.open>) => {
      calls.push(args);
      return open(...args);
    };
  }, WINDOW_OPEN);
}

function windowOpenCalls(page: Page): Promise<unknown[][]> {
  return page.evaluate((name) => Reflect.get(window, name) as unknown[][], WINDOW_OPEN);
}

test.use({ signedIn: true });

test.beforeEach(async ({ context }) => {
  await watchWindowOpen(context);
});

test('WEB-011, D-5: the Web button of one unpinned interface opens a new tab straight to the device, with no opener and no referrer', async ({
  page,
  context,
}) => {
  const device = await serveDevicePages(context);
  await openDevices(page);
  const opened = context.waitForEvent('page');
  await page.getByRole('button', { name: 'Open Dashboard of mac-mini' }).click();
  const tab = await opened;
  await tab.waitForLoadState('domcontentloaded');

  expect(await windowOpenCalls(page)).toEqual([[MAC_MINI_PAGE, '_blank', 'noopener,noreferrer']]);
  expect(tab.url()).toBe(MAC_MINI_PAGE);
  expect(await tab.evaluate(() => [window.opener, document.referrer])).toEqual([null, '']);
  expect(device.referers).toEqual(device.referers.map(() => ''));
  expect(device.referers.length).toBeGreaterThan(0);
  // The client's own page stays where it was: no web view tab replaced it, and nothing was added to its session strip.
  expect(page.url()).toBe('https://hub.anima.ing/devices');
  await expect(page.getByRole('tab')).toHaveCount(0);
});

test('WEB-011: only http and https addresses open: an ftp interface opens nothing, and the next http one still opens its tab', async ({
  page,
  context,
}) => {
  await serveDevicePages(context);
  await openDevices(page);
  await page.getByRole('button', { name: 'Open Files of win-legacy' }).click();
  // The click above ran first and synchronously; this one opens a tab, so any tab the ftp click made would be counted too.
  const opened = context.waitForEvent('page');
  await page.getByRole('button', { name: 'Open Dashboard of mac-mini' }).click();
  await opened;

  expect(await windowOpenCalls(page)).toEqual([[MAC_MINI_PAGE, '_blank', 'noopener,noreferrer']]);
  expect(context.pages()).toHaveLength(2);
});

test('WEB-011, D-5: with a pinned certificate the button opens a menu, each choice has its own pin tied by aria-describedby, and a choice opens its tab', async ({
  page,
  context,
}) => {
  await serveDevicePages(context);
  await openDevices(page);
  const button = page.getByRole('button', { name: 'Web interfaces of nas' });
  await expect(button).toHaveAttribute('aria-expanded', 'false');
  await button.click();
  await expect(button).toHaveAttribute('aria-expanded', 'true');

  const menu = page.getByRole('group', { name: 'Web interfaces of nas' });
  const admin = menu.getByRole('button', { name: /^Admin/ });
  const files = menu.getByRole('button', { name: /^Files/ });
  // The pin is outside the choice's name, in its own element, which the choice names as its description.
  const pinId = await admin.getAttribute('aria-describedby');
  expect(pinId).toBeTruthy();
  await expect(page.locator(`[id="${pinId ?? ''}"]`)).toContainText(NAS_ADMIN_PIN);
  await expect(admin).toHaveAccessibleDescription(new RegExp(NAS_ADMIN_PIN));
  await expect(admin).not.toHaveAccessibleName(new RegExp(NAS_ADMIN_PIN));
  // The unpinned choice has no pin and no description to borrow another's.
  await expect(files).not.toHaveAttribute('aria-describedby', /.+/);
  await expect(files).toHaveAccessibleDescription('');
  await expect(menu.getByText(NAS_ADMIN_PIN)).toHaveCount(1);
  expect(await windowOpenCalls(page)).toEqual([]);

  const opened = context.waitForEvent('page');
  await admin.click();
  const tab = await opened;
  await tab.waitForLoadState('domcontentloaded');
  expect(tab.url()).toBe(NAS_ADMIN_PAGE);
  expect(await tab.evaluate(() => [window.opener, document.referrer])).toEqual([null, '']);
  expect(await windowOpenCalls(page)).toEqual([[NAS_ADMIN_PAGE, '_blank', 'noopener,noreferrer']]);
  await expect(menu).toHaveCount(0);
});

test('WEB-011: Escape closes the menu and puts focus back on the Web button', async ({ page }) => {
  await openDevices(page);
  const button = page.getByRole('button', { name: 'Web interfaces of nas' });
  await button.click();
  await expect(page.getByRole('group', { name: 'Web interfaces of nas' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('group', { name: 'Web interfaces of nas' })).toHaveCount(0);
  await expect(button).toBeFocused();
});
