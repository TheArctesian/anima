import { expect, test } from '../test-hub';
import { openDevices } from './screens';

// The Web menu is fixed where it opened, so a real scroll that moves its button closes it (Section 5.7.3, MOT-002); an event
// that moves nothing does not: Chromium fires a document `scroll` about 130 ms after a click with no scroll input, and a mobile
// address bar fires `resize` without changing the width (WEB-011). Run in the real engines, where those events are the browser's own.

const MENU = { name: 'Web interfaces of nas' };
const STAYS_OPEN_MS = 500;

test.use({ signedIn: true });

test('WEB-011: the menu stays open for 500 ms after a plain click', async ({ page }) => {
  await openDevices(page);
  await page.getByRole('button', MENU).click();
  const menu = page.getByRole('group', MENU);
  await expect(menu).toBeVisible();
  await page.waitForTimeout(STAYS_OPEN_MS);
  await expect(menu).toBeVisible();
});

test('MOT-002: a real wheel scroll that moves the button closes the menu', async ({ page }) => {
  await openDevices(page);
  // The page must be able to scroll by more than the tolerance, whatever the fixtures hold.
  await page.addStyleTag({ content: 'body::after { content: ""; display: block; block-size: 3000px; }' });
  const button = page.getByRole('button', MENU);
  await button.scrollIntoViewIfNeeded();
  await button.click();
  const menu = page.getByRole('group', MENU);
  await expect(menu).toBeVisible();
  const before = await button.boundingBox();
  await page.mouse.move(5, 5);
  await page.mouse.wheel(0, 200);
  await expect.poll(async () => (await button.boundingBox())?.y).not.toBe(before?.y);
  await expect(menu).toHaveCount(0);
});

test('WEB-011: a resize that keeps the width keeps the menu open, and a width change closes it', async ({ page }) => {
  await openDevices(page);
  const { width, height } = page.viewportSize() ?? { width: 1280, height: 720 };
  await page.getByRole('button', MENU).click();
  const menu = page.getByRole('group', MENU);
  await expect(menu).toBeVisible();

  await page.setViewportSize({ width, height: height - 80 });
  await page.waitForTimeout(STAYS_OPEN_MS);
  await expect(menu).toBeVisible();

  await page.setViewportSize({ width: width - 120, height: height - 80 });
  await expect(menu).toHaveCount(0);
});
