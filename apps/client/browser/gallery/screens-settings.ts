import { expect } from '@playwright/test';
import type { Screen } from './shots';
import { openedDialog, openSettings, section } from './steps';

// The Settings page: the whole page, each section on its own, a scrolled window and the dialogs the sections open.
// The Updates section is desktop only (UPD-001), so the web client the gallery drives never shows it.

const SECTIONS = ['Account', 'Appearance', 'Notifications', 'Default check settings', 'Data retention', 'API keys', 'VPN'] as const;

function sectionScreen(name: (typeof SECTIONS)[number]): Screen {
  return {
    name: `settings-${name.toLowerCase().replaceAll(' ', '-')}`,
    open: openSettings,
    target: (page) => section(page, name),
  };
}

function channelDialog(slug: string, channel: string): Screen {
  return {
    name: `settings-edit-channel-${slug}`,
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: `Edit ${channel}`, exact: true }).click();
      await openedDialog(page);
    },
    target: (page) => page.getByRole('dialog'),
  };
}

export const SETTINGS_SCREENS: readonly Screen[] = [
  { name: 'settings-page', open: openSettings },
  ...SECTIONS.map(sectionScreen),
  {
    name: 'settings-scrolled-to-end',
    open: async (page) => {
      await openSettings(page);
      await section(page, 'VPN').scrollIntoViewIfNeeded();
      await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    },
    windowOnly: true,
  },
  {
    name: 'settings-scrolled-to-notifications',
    open: async (page) => {
      await openSettings(page);
      await section(page, 'Notifications').scrollIntoViewIfNeeded();
    },
    windowOnly: true,
  },
  {
    name: 'settings-add-channel',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Add channel' }).click();
      await openedDialog(page);
    },
    target: (page) => page.getByRole('dialog'),
  },
  channelDialog('webhook', 'ops-webhook'),
  channelDialog('email', 'ops-email'),
  channelDialog('ntfy', 'phone-ntfy'),
  channelDialog('apprise', 'apprise-bridge'),
  {
    name: 'settings-delete-channel-confirm',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Delete ops-webhook' }).click();
      await openedDialog(page);
    },
    windowOnly: true,
  },
  {
    name: 'settings-revoke-key-confirm',
    open: async (page) => {
      await openSettings(page);
      await page.getByRole('button', { name: 'Revoke metrics-scrape' }).click();
      await openedDialog(page);
    },
    windowOnly: true,
  },
  {
    name: 'settings-retention-invalid',
    open: async (page) => {
      await openSettings(page);
      const field = page.getByRole('spinbutton', { name: 'Events' });
      await field.fill('1');
      await page.getByRole('button', { name: 'Save Events' }).click();
      await expect(field).toBeVisible();
    },
    target: (page) => section(page, 'Data retention'),
  },
];
