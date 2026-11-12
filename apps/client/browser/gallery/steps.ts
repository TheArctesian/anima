import { expect, type Page, type Route } from '@playwright/test';
import { HUB_ORIGIN } from '../synthetic/access';
import { DEVICE_IDS as FIXTURE_DEVICES, openDevices } from '../acceptance/screens';

// The steps screens share: each opens a route and waits for the thing that shows the page is ready.

export { openDevices };
export const DEVICE_IDS = { ...FIXTURE_DEVICES, winDesktop: '0190a000-0000-7000-8012-000000000001' } as const;

export async function openDevice(page: Page, id: string, name: string): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/devices/${id}`);
  await expect(page.getByRole('heading', { level: 1, name })).toBeVisible();
}

export async function openGpuBox(page: Page, tab?: string): Promise<void> {
  await openDevice(page, DEVICE_IDS.gpuBox, 'gpu-box');
  if (tab !== undefined) await page.getByRole('tab', { name: tab }).click();
}

export async function openSettings(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/settings`);
  await expect(page.getByRole('heading', { level: 2, name: 'API keys' })).toBeVisible();
  await expect(page.getByText('router-lab').first()).toBeVisible();
}

/** The Settings section named `name`. */
export const section = (page: Page, name: string) => page.getByRole('region', { name, exact: true });

/** The dialog open now, after its open motion. */
export async function openedDialog(page: Page, name?: string) {
  const dialog = name === undefined ? page.getByRole('dialog') : page.getByRole('dialog', { name });
  await expect(dialog.first()).toBeVisible();
  return dialog.first();
}

/** Runs `answer` for each `POST /graphql` named in `operations`; every other call goes on to the synthetic hub. */
export async function onOperations(page: Page, operations: readonly string[], answer: (route: Route) => Promise<void>): Promise<void> {
  await page.route('**/graphql', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as { query?: string; operationName?: string };
    const name = body.operationName ?? /^\s*(?:query|mutation)\s+(\w+)/.exec(body.query ?? '')?.[1] ?? '';
    if (route.request().method() === 'POST' && operations.includes(name)) await answer(route);
    else await route.fallback();
  });
}

/** The hub answers these operations with a GraphQL error. */
export function failOperations(page: Page, operations: readonly string[]): Promise<void> {
  const body = JSON.stringify({ data: null, errors: [{ message: 'The hub could not read its database.', extensions: { code: 'INTERNAL' } }] });
  return onOperations(page, operations, (route) => route.fulfill({ status: 200, contentType: 'application/graphql-response+json', body }));
}

/** The hub never answers these operations, so the page stays in its loading state. */
export function holdOperations(page: Page, operations: readonly string[]): Promise<void> {
  return onOperations(page, operations, () => new Promise<void>(() => undefined));
}
