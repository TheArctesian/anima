import { expect, type Page } from '@playwright/test';
import type { Screen } from './shots';
import { openDevices, openedDialog } from './steps';

// The device list: its two views, each filter, the logs panel, the live sessions popover and Add device.

const toolbar = (page: Page) => page.getByRole('search', { name: 'Filter devices' });

/** Opens the toolbar menu `name` and ticks its first choice. */
async function chooseFirst(page: Page, name: string): Promise<void> {
  await toolbar(page).getByRole('button', { name, exact: true }).click();
  const panel = page.getByRole('group', { name, exact: true });
  await expect(panel).toBeVisible();
  await panel.locator('input').first().click({ force: true });
}

async function openMenu(page: Page, name: string): Promise<void> {
  await toolbar(page).getByRole('button', { name, exact: true }).click();
  await expect(page.getByRole('group', { name, exact: true })).toBeVisible();
}

async function openAddDevice(page: Page): Promise<void> {
  await openDevices(page);
  await toolbar(page).getByRole('button', { name: 'Add device' }).click();
  await openedDialog(page, 'Add device');
}

/** Add device with the form section `name` opened, shot as the whole dialog because the section is taller than the window. */
function addDeviceSection(name: string, slug: string): Screen {
  return {
    name: `add-device-${slug}`,
    open: async (page) => {
      await openAddDevice(page);
      await page.getByRole('button', { name, exact: true }).click();
    },
    target: (page) => page.getByRole('dialog', { name: 'Add device' }),
  };
}

export const DEVICE_LIST_SCREENS: readonly Screen[] = [
  { name: 'devices-grid', open: openDevices, windowOnly: true },
  {
    name: 'devices-grid-scrolled-to-end',
    open: async (page) => {
      await openDevices(page);
      await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    },
    windowOnly: true,
  },
  {
    name: 'devices-table',
    open: async (page) => {
      await openDevices(page);
      await toolbar(page).getByRole('button', { name: 'Table', exact: true }).click();
      await expect(page.getByRole('table')).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'devices-nav-collapsed',
    open: async (page) => {
      await openDevices(page);
      await page.getByRole('button', { name: 'Collapse sidebar' }).click();
    },
    windowOnly: true,
  },
  {
    name: 'devices-search',
    open: async (page) => {
      await openDevices(page);
      await toolbar(page).getByRole('searchbox').fill('mac');
      await page.keyboard.press('Enter');
      await page.waitForTimeout(800);
    },
    windowOnly: true,
  },
  {
    name: 'devices-search-no-match',
    open: async (page) => {
      await openDevices(page);
      await toolbar(page).getByRole('searchbox').fill('no such device');
      await page.keyboard.press('Enter');
      await page.waitForTimeout(800);
    },
    windowOnly: true,
  },
  ...['Status', 'Tags', 'Site', 'Sort'].map((name): Screen => ({
    name: `filter-${name.toLowerCase()}-menu`,
    open: async (page) => {
      await openDevices(page);
      await openMenu(page, name);
    },
    windowOnly: true,
  })),
  ...['Status', 'Tags', 'Site', 'Sort'].map((name): Screen => ({
    name: `filter-${name.toLowerCase()}-applied`,
    open: async (page) => {
      await openDevices(page);
      await chooseFirst(page, name);
    },
    windowOnly: true,
  })),
  {
    name: 'devices-logs-collapsed',
    open: async (page) => {
      await openDevices(page);
      await page.getByRole('region', { name: 'Logs' }).getByRole('button', { name: 'Collapse' }).click();
    },
    windowOnly: true,
  },
  {
    name: 'devices-logs-filter-severity',
    open: async (page) => {
      await openDevices(page);
      await page.getByRole('group', { name: 'Log filters' }).getByRole('button', { name: 'Severity' }).click();
    },
    windowOnly: true,
  },
  {
    name: 'live-sessions-popover',
    open: async (page) => {
      await openDevices(page);
      await page.getByRole('button', { name: /^Live sessions/ }).click();
      await expect(page.getByRole('group', { name: 'Live sessions' })).toBeVisible();
    },
    windowOnly: true,
  },
  { name: 'add-device', open: openAddDevice, windowOnly: true },
  {
    name: 'add-device-invalid',
    open: async (page) => {
      await openAddDevice(page);
      await page.getByRole('button', { name: 'Save', exact: true }).click();
    },
    windowOnly: true,
  },
  addDeviceSection('General', 'general-closed'),
  addDeviceSection('SSH', 'ssh-closed'),
  addDeviceSection('Remote Desktop (RDP)', 'rdp-open'),
  addDeviceSection('VNC', 'vnc-open'),
  addDeviceSection('Sessions', 'sessions-open'),
  addDeviceSection('Files and clipboard', 'files-open'),
];
