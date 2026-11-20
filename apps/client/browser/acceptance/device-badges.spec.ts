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
