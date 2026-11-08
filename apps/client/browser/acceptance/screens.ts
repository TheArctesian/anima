import { expect, type Page } from '@playwright/test';
import { HUB_ORIGIN } from '../synthetic/access';

// Where the acceptance specs go, and the fixture devices they use (`contracts/fixtures`, plus `synthetic/device-additions.ts`).

export const DEVICE_IDS = {
  winLegacy: '0190a000-0000-7000-8012-000000000002',
  macMini: '0190a000-0000-7000-8012-000000000003',
  gpuBox: '0190a000-0000-7000-8012-000000000004',
  nas: '0190a000-0000-7000-8012-000000000005',
} as const;

/**
 * The device list, once every fixture device and the logs panel's rows show.
 */
export async function openDevices(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/devices`, { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Logs' }).getByText('Sign-in refused', { exact: true })).toBeVisible();
}
