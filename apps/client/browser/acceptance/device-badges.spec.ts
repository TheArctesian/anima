import type { Locator } from '@playwright/test';
import { expect, test } from '../test-hub';
import { DEVICE_IDS, openDevices } from './screens';

test.use({ signedIn: true });

test('DEV-007: the wide device list gives the name and address separate lines', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openDevices(page);
  await page.getByRole('button', { name: 'List', exact: true }).click();
  const table = page.getByRole('table', { name: 'Devices' });
  const row = table.getByRole('row', { name: /mac-mini/ });
  const identity = row.getByRole('rowheader');
  const name = identity.getByRole('link', { name: 'mac-mini', exact: true });
  const address = identity.getByText('10.40.0.13', { exact: true });
  await expect(address).toBeVisible();
  await expect(table.getByRole('columnheader', { name: 'Last 50 checks' })).toBeVisible();

  const [rowBox, nameBox, addressBox] = await Promise.all([row.boundingBox(), name.boundingBox(), address.boundingBox()]);
  if (rowBox === null || nameBox === null || addressBox === null) throw new Error('The device row is missing its name or address');
  expect(rowBox.height).toBeGreaterThanOrEqual(56);
  expect(nameBox.y + nameBox.height).toBeLessThanOrEqual(addressBox.y);
  expect(rowBox.x + rowBox.width).toBeLessThanOrEqual(1280);

  // The words of the usage are in the column head, whole; a row has the values only, so each row has one height.
  const memory = table.getByRole('columnheader', { name: 'Usage: CPU, memory, disk' }).getByText('Memory', { exact: true });
  await expect(memory).toBeVisible();
  expect(await memory.evaluate((label) => label.scrollWidth)).toBeLessThanOrEqual(await memory.evaluate((label) => label.clientWidth));
  await expect(table.getByRole('row', { name: /gpu-box/ }).getByRole('meter', { name: 'Memory' })).toBeVisible();
  const heights = await Promise.all(['gpu-box', 'win-desktop', 'mac-mini'].map(async (device) => (await table.getByRole('row', { name: new RegExp(device) }).boundingBox())?.height));
  expect(new Set(heights).size).toBe(1);
});

test('DEV-007, RA-020: status and warning sit side by side inside the Status column', async ({ page, hub }) => {
  await openDevices(page);
  await page.getByRole('button', { name: 'List', exact: true }).click();
  const row = page.getByRole('row', { name: /mac-mini/ });
  const status = row.getByRole('cell').filter({ hasText: 'Warning' });
  const warning = status.getByTitle('VNC is not encrypted. Use it only over the VPN.');
  const name = row.getByRole('rowheader');
  await expect(status.getByText('Up', { exact: true })).toBeVisible();
  await expect.poll(() => hub.recording.where({ channel: 'graphql-socket', operation: 'DeviceStatusChanged' }).length).toBe(1);
  expect(hub.push('DeviceStatusChanged', {
    deviceStatusChanged: { deviceId: DEVICE_IDS.macMini, status: 'MAINTENANCE', at: '2026-11-15T20:00:00Z' },
  })).toBe(1);
  const badge = status.getByText('Maintenance', { exact: true });

  for (const width of [1280, 1101]) {
    await page.setViewportSize({ width, height: 800 });
    await expect(badge).toBeVisible();
    await expect(warning).toBeVisible();
    await page.evaluate(async () => {
      await document.fonts.ready;
    });
    await expect(async () => {
      const [column, statusBox, warningBox] = await Promise.all([name.boundingBox(), badge.boundingBox(), warning.boundingBox()]);
      if (column === null || statusBox === null || warningBox === null) throw new Error('Device row is missing a badge or name');
      expect(statusBox.x + statusBox.width).toBeLessThanOrEqual(warningBox.x);
      expect(Math.abs(statusBox.y + statusBox.height / 2 - warningBox.y - warningBox.height / 2)).toBeLessThanOrEqual(1);
      // The name is the first column and the status the second (R-224): the two marks stay right of the name.
      expect(column.x + column.width).toBeLessThanOrEqual(statusBox.x);
    }).toPass({ timeout: 5000 });
  }
});

// A panel that opens from a row of the Devices list is inside a table cell that does not wrap its text. The panel
// must wrap its own sentences and stay inside the window, at any width.

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

/** The top of `target` once the entrance of the page has ended: the head rises 4 px as it enters (MOT-053). */
async function settledTop(target: Locator): Promise<number> {
  let last = Number.NaN;
  await expect(async () => {
    const top = (await target.boundingBox())?.y ?? Number.NaN;
    const still = top === last;
    last = top;
    expect(still).toBe(true);
  }).toPass({ intervals: [150], timeout: 5000 });
  return last;
}

test('MOT-007: a device that goes down keeps the height of its row, and the rows below do not move', async ({ page, hub }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openDevices(page);
  await page.getByRole('button', { name: 'List', exact: true }).click();
  const table = page.getByRole('table', { name: 'Devices' });
  const row = table.getByRole('row', { name: /mac-mini/ });
  const below = table.getByRole('row', { name: /mac-studio/ });
  await settledTop(row);
  const before = await Promise.all([row.boundingBox(), below.boundingBox(), table.getByRole('row', { name: /win-desktop/ }).boundingBox()]);
  await expect.poll(() => hub.recording.where({ channel: 'graphql-socket', operation: 'DeviceStatusChanged' }).length).toBe(1);
  expect(hub.push('DeviceStatusChanged', {
    deviceStatusChanged: { deviceId: DEVICE_IDS.macMini, status: 'DOWN', at: '2026-11-15T20:00:00Z' },
  })).toBe(1);
  // Down: VNC, Web and now Wake, on one line.
  await expect(row.getByRole('button', { name: 'Wake' })).toBeVisible();
  await expect(row.getByRole('button', { name: 'VNC to mac-mini' })).toBeVisible();
  await settledTop(row);
  const after = await row.boundingBox();
  const same = (a: number | undefined, b: number | undefined): boolean => a !== undefined && b !== undefined && Math.abs(a - b) < 0.5;
  expect(same(after?.height, before[0]?.height)).toBe(true);
  expect(same(after?.height, before[2]?.height)).toBe(true);
  // The group "Needs a look" is new above the rows that stay up; a row that stays up keeps its own height.
  expect(same((await below.boundingBox())?.height, before[1]?.height)).toBe(true);
});

test('MOT-007: opening a menu of the toolbar with the pointer does not scroll the page', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openDevices(page);
  const toolbar = page.getByRole('search', { name: 'Filter devices' });
  for (const name of ['Tags', 'Site', 'Sort: Status']) {
    const button = toolbar.getByRole('button', { name, exact: true });
    const top = await settledTop(button);
    const box = await button.boundingBox();
    if (box === null) throw new Error(`The toolbar has no ${name} button`);
    // The pointer itself, where a person clicks: `click()` first scrolls its target to a place of its own choice.
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(toolbar.getByRole('group', { name: name.replace(': Status', ''), exact: true })).toBeVisible();
    expect(await page.evaluate(() => window.scrollY)).toBe(0);
    expect((await button.boundingBox())?.y).toBe(top);
    await page.keyboard.press('Escape');
    await expect(button).toHaveAttribute('aria-expanded', 'false');
  }
});

test('the count line does not say "0 devices" while the list loads, and keeps its height', async ({ page }) => {
  let release = (): void => undefined;
  const held = new Promise<void>((resolve) => { release = resolve; });
  await page.route('**/graphql', async (route) => {
    const body = route.request().postDataJSON() as { query?: string } | null;
    if (body?.query?.startsWith('query Devices') === true) await held;
    await route.fallback();
  });
  await page.goto('/devices', { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('status').filter({ hasText: 'Loading the devices' })).toBeAttached();
  await expect(page.getByText('0 devices')).toHaveCount(0);
  const head = page.getByRole('search', { name: 'Filter devices' });
  const top = await settledTop(head);
  release();
  await expect(page.getByRole('group', { name: 'Show devices by status' }).getByRole('button', { name: /8\s*devices/ })).toBeVisible();
  expect(await settledTop(head)).toBe(top);
});
