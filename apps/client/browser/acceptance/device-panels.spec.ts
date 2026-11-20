import { expect, test } from '../test-hub';
import { openDevices } from './screens';

// A panel that opens from a row of the Devices list is inside a table cell that does not wrap its text. The panel
// must wrap its own sentences and stay inside the window, at any width.

test.use({ signedIn: true });

for (const width of [1600, 1280, 820]) {
  test(`R-079: the panel of "Set up screen sharing" wraps its text and stays inside a window ${String(width)} px wide`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    await openDevices(page);
    await page.getByRole('button', { name: 'Set up screen sharing for nas' }).click();
    const panel = page.locator('.setup .panel');
    await expect(panel.getByRole('alert')).toBeVisible();
    const box = await panel.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      return { left: rect.left, right: rect.right, textFits: element.scrollWidth <= element.clientWidth, wraps: getComputedStyle(element).whiteSpace };
    });
    expect(box.wraps).toBe('normal');
    expect(box.textFits).toBe(true);
    expect(box.left).toBeGreaterThanOrEqual(0);
    expect(box.right).toBeLessThanOrEqual(width);
  });
}

test('R-224: on a wide window the name column takes the free width, so a name and its address are whole', async ({ page }) => {
  await page.setViewportSize({ width: 1800, height: 900 });
  await openDevices(page);
  const widths = await page.locator('.device-rows').evaluate((table) => {
    const width = (selector: string) => table.querySelector(selector)?.getBoundingClientRect().width ?? 0;
    const cut = [...table.querySelectorAll<HTMLElement>('.c-name a, .c-name .host')].filter((each) => each.scrollWidth > each.clientWidth).length;
    return { table: table.getBoundingClientRect().width, name: width('thead .c-name'), usage: width('thead .c-detail'), cut };
  });
  expect(widths.name).toBeGreaterThan(widths.table * 0.3);
  expect(widths.usage).toBeLessThanOrEqual(336);
  expect(widths.cut).toBe(0);
});
