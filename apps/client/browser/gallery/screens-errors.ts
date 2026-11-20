import { expect } from '@playwright/test';
import { HUB_ORIGIN } from '../synthetic/access';
import type { Screen } from './shots';
import { DEVICE_IDS, failOperations, openDevice } from './steps';

// The failures of the other screens: an address the app does not have, a project that is not there, and the hub
// failing on Projects, Agent usage, Extensions and Containers. They come last, so the numbers before them stay.

export const ERROR_SCREENS: readonly Screen[] = [
  {
    name: 'page-not-found',
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/no-such-page`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'project-not-found',
    open: async (page) => {
      await page.goto(`${HUB_ORIGIN}/projects/no-such-project`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'projects-hub-error',
    open: async (page) => {
      await failOperations(page, ['Projects']);
      await page.goto(`${HUB_ORIGIN}/projects`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'agent-usage-hub-error',
    open: async (page) => {
      await failOperations(page, ['AgentUsage']);
      await page.goto(`${HUB_ORIGIN}/agents/usage`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'extensions-hub-error',
    open: async (page) => {
      await failOperations(page, ['Extensions']);
      await page.goto(`${HUB_ORIGIN}/extensions`);
      await page.waitForTimeout(1500);
    },
  },
  {
    name: 'containers-hub-error',
    open: async (page) => {
      await failOperations(page, ['Containers']);
      await openDevice(page, DEVICE_IDS.gpuBox, 'gpu-box');
      const tab = page.getByRole('tab', { name: 'Containers' });
      if (await tab.count()) await tab.click();
      await expect(page.getByRole('main')).toBeVisible();
      await page.waitForTimeout(1500);
    },
  },
];
