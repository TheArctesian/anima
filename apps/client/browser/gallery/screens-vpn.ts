import { expect, type Page } from '@playwright/test';
import type { Screen } from './shots';
import { failOperations, holdOperations, openedDialog, openSettings } from './steps';

// The VPN section's dialogs: add a computer by either path, add a router, approve a route, renew a key.

async function addComputer(page: Page): Promise<void> {
  await openSettings(page);
  await page.getByRole('button', { name: 'Add a computer to the VPN' }).click();
  await openedDialog(page);
}

const dialog = (page: Page) => page.getByRole('dialog');

export const VPN_SCREENS: readonly Screen[] = [
  { name: 'vpn-add-computer', open: addComputer, windowOnly: true },
  {
    name: 'vpn-add-computer-linux-key',
    open: async (page) => {
      await addComputer(page);
      await page.getByRole('button', { name: 'Linux computer' }).click();
      await expect(dialog(page).getByRole('button', { name: 'Copy' }).first()).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'vpn-add-computer-register',
    open: async (page) => {
      await addComputer(page);
      await page.getByRole('button', { name: 'Mac or other computer' }).click();
    },
    windowOnly: true,
  },
  {
    name: 'vpn-add-computer-registered',
    open: async (page) => {
      await addComputer(page);
      await page.getByRole('button', { name: 'Mac or other computer' }).click();
      await dialog(page).getByRole('textbox').fill('AUTH-1234');
      await page.getByRole('button', { name: 'Register', exact: true }).click();
      await expect(dialog(page).getByText('mac-studio').first()).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'vpn-add-router',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Add a router' }).first().click();
      await openedDialog(page);
    },
    windowOnly: true,
  },
  {
    name: 'vpn-add-router-key',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Add a router' }).first().click();
      await openedDialog(page);
      await dialog(page).getByRole('button', { name: /key/i }).first().click();
      await expect(dialog(page).getByRole('button', { name: 'Copy' }).first()).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'vpn-approve-route-confirm',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Approve route' }).click();
      await openedDialog(page);
    },
    windowOnly: true,
  },
  {
    name: 'vpn-unavailable-error',
    open: async (page) => {
      await failOperations(page, ['Vpn']);
      await page.goto('https://hub.anima.ing/settings');
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'vpn-loading',
    loading: true,
    open: async (page) => {
      await holdOperations(page, ['Vpn']);
      await page.goto('https://hub.anima.ing/settings');
      await expect(page.getByRole('heading', { level: 2, name: 'VPN' })).toBeVisible();
    },
  },
];
