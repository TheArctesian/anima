import { expect, test } from '../test-hub';
import { DEVICE_IDS, openDevices } from './screens';

test.use({ signedIn: true });

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
      expect(warningBox.x + warningBox.width).toBeLessThanOrEqual(column.x);
    }).toPass({ timeout: 5000 });
  }
});
