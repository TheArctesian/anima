import { expect } from '@playwright/test';
import { HUB_ORIGIN } from '../synthetic/access';
import type { Fixtures } from '../synthetic/fixtures';
import type { Screen } from './shots';
import { DEVICE_IDS, failOperations, holdOperations, onOperations, openDevice } from './steps';

// The states between the good ones: sign-in, nothing to show, still loading, and the hub failing or gone.

const noDevices = (loaded: Fixtures): Fixtures => ({ ...loaded, devices: [], checks: [], sessions: [], metrics: [] });
const noChecks = (loaded: Fixtures): Fixtures => ({ ...loaded, checks: [] });
const noChannels = (loaded: Fixtures): Fixtures => ({ ...loaded, channels: [], deliveries: [] });
const noKeys = (loaded: Fixtures): Fixtures => ({ ...loaded, apiKeys: [] });

const EMPTY_PAGE_INFO = { hasNextPage: false, endCursor: null };

export const SIGN_IN_SCREENS: readonly Screen[] = [
  {
    name: 'sign-in',
    signedOut: true,
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/sign-in`);
      await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeEnabled();
    },
  },
  {
    name: 'sign-in-refused',
    signedOut: true,
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/sign-in`);
      await page.getByLabel('User name', { exact: true }).fill('ann');
      await page.getByLabel('Password', { exact: true }).fill('wrong');
      await page.getByLabel('Code from your authenticator', { exact: true }).fill('000000');
      await page.getByRole('button', { name: 'Sign in', exact: true }).click();
      await expect(page.getByRole('alert')).toBeVisible();
    },
  },
];

export const STATE_SCREENS: readonly Screen[] = [
  {
    name: 'devices-empty',
    fixtures: noDevices,
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/devices`);
      await expect(page.getByRole('heading', { level: 1, name: 'Devices' })).toBeAttached();
      await expect(page.getByRole('button', { name: 'Add device' }).first()).toBeVisible();
    },
  },
  {
    name: 'device-no-checks',
    fixtures: noChecks,
    open: async (page) => {
      await openDevice(page, DEVICE_IDS.gpuBox, 'gpu-box');
      await page.getByRole('tab', { name: 'Checks' }).click();
      await expect(page.getByRole('tabpanel', { name: 'Checks' })).toBeVisible();
    },
  },
  {
    name: 'settings-no-channels',
    fixtures: noChannels,
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/settings`);
      await expect(page.getByRole('heading', { level: 3, name: 'Channels' })).toBeVisible();
    },
  },
  {
    name: 'settings-no-api-keys',
    fixtures: noKeys,
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/settings`);
      await expect(page.getByRole('heading', { level: 2, name: 'API keys' })).toBeVisible();
    },
  },
  {
    name: 'logs-empty',
    open: async (page) => {
      const empty = { totalCount: 0, edges: [], pageInfo: EMPTY_PAGE_INFO };
      await onOperations(page, ['Events'], (route) =>
        route.fulfill({ contentType: 'application/graphql-response+json', body: JSON.stringify({ data: { events: empty } }) }),
      );
      await page.goto(`${HUB_ORIGIN}/devices`);
      await expect(page.getByRole('region', { name: 'Logs' })).toBeVisible();
      await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
    },
  },
  {
    name: 'devices-loading',
    loading: true,
    open: async (page) => {
      await holdOperations(page, ['Devices', 'Events']);
      await page.goto(`${HUB_ORIGIN}/devices`);
      await expect(page.getByRole('heading', { level: 1, name: 'Devices' })).toBeAttached();
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'device-loading',
    loading: true,
    open: async (page) => {
      await holdOperations(page, ['Device']);
      await page.goto(`${HUB_ORIGIN}/devices/${DEVICE_IDS.gpuBox}`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'settings-loading',
    loading: true,
    open: async (page) => {
      await holdOperations(page, ['ApiKeys', 'NotificationChannels', 'NotificationSettings']);
      await page.goto(`${HUB_ORIGIN}/settings`);
      await expect(page.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'devices-hub-error',
    open: async (page) => {
      await failOperations(page, ['Devices']);
      await page.goto(`${HUB_ORIGIN}/devices`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'device-hub-error',
    open: async (page) => {
      await failOperations(page, ['Device']);
      await page.goto(`${HUB_ORIGIN}/devices/${DEVICE_IDS.gpuBox}`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'settings-hub-error',
    open: async (page) => {
      await failOperations(page, ['ApiKeys', 'NotificationChannels', 'NotificationSettings']);
      await page.goto(`${HUB_ORIGIN}/settings`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'devices-hub-unreachable',
    open: async (page) => {
      await onOperations(page, ['Devices', 'Events'], (route) => route.abort('connectionfailed'));
      await page.goto(`${HUB_ORIGIN}/devices`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'devices-socket-offline',
    open: async (page) => {
      await page.routeWebSocket((url) => url.pathname === '/graphql', (socket) => void socket.close({ code: 1006, reason: 'Gone' }));
      await page.goto(`${HUB_ORIGIN}/devices`);
      await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'sign-in-hub-unreachable',
    signedOut: true,
    open: async (page) => {
      await page.routeWebSocket((url) => url.pathname === '/graphql', (socket) => void socket.close({ code: 1006, reason: 'Gone' }));
      await page.goto(`${HUB_ORIGIN}/sign-in`);
      await expect(page.getByText("Can't reach")).toBeVisible();
    },
  },
];
