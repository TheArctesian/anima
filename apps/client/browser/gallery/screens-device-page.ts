import { expect } from '@playwright/test';
import type { Screen } from './shots';
import { DEVICE_IDS, openDevice, openGpuBox, openedDialog } from './steps';

// One device's page: its five tabs and the dialogs and states on them.

const NO_SUCH_DEVICE = '0190a000-0000-7000-8012-0000000000ff';

export const DEVICE_PAGE_SCREENS: readonly Screen[] = [
  { name: 'device-overview', open: (page) => openGpuBox(page) },
  {
    name: 'device-overview-metrics-24h',
    open: async (page) => {
      await openGpuBox(page);
      await page.getByRole('button', { name: '24 hours', exact: true }).click();
    },
  },
  { name: 'device-overview-no-metrics', open: (page) => openDevice(page, DEVICE_IDS.winDesktop, 'win-desktop') },
  { name: 'device-overview-warning', open: (page) => openDevice(page, DEVICE_IDS.winLegacy, 'win-legacy') },
  { name: 'device-overview-mac', open: (page) => openDevice(page, DEVICE_IDS.macMini, 'mac-mini') },
  { name: 'device-checks', open: (page) => openGpuBox(page, 'Checks') },
  {
    name: 'device-check-history',
    open: async (page) => {
      await openGpuBox(page, 'Checks');
      await page.getByRole('button', { name: 'Container ollama', exact: true }).click();
    },
  },
  {
    name: 'device-add-check',
    open: async (page) => {
      await openGpuBox(page, 'Checks');
      await page.getByRole('button', { name: 'Add check' }).click();
      await openedDialog(page, 'Add check');
    },
    windowOnly: true,
  },
  {
    name: 'device-edit-check',
    open: async (page) => {
      await openGpuBox(page, 'Checks');
      await page.getByRole('button', { name: 'Edit Container ollama' }).click();
      await openedDialog(page);
    },
    windowOnly: true,
  },
  { name: 'device-sessions', open: (page) => openGpuBox(page, 'Sessions') },
  { name: 'device-logs', open: (page) => openGpuBox(page, 'Logs') },
  { name: 'device-settings', open: (page) => openGpuBox(page, 'Settings') },
  {
    name: 'device-edit',
    open: async (page) => {
      await openGpuBox(page);
      await page.getByRole('button', { name: 'Edit', exact: true }).click();
      await page.waitForTimeout(800);
    },
    windowOnly: true,
  },
  {
    name: 'device-missing',
    open: async (page) => {
      await page.goto(`https://hub.anima.ing/devices/${NO_SUCH_DEVICE}`);
      await expect(page.getByRole('main')).toContainText(/not|exist|found/i);
    },
  },
];
