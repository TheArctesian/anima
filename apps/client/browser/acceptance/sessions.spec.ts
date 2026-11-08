import { expect, test } from '../test-hub';
import { k7Policy } from '../csp/k7-policy';
import { expectNoViolations, servePolicy, violationsSoFar, type PolicyReports } from '../csp/violations';
import { TERMINAL_TITLE } from '../synthetic/tunnels';
import { openDevices } from './screens';

// Plan criterion 2, the live sessions, client side: one SSH session (xterm.js) and one VNC session (Guacamole) open over
// the synthetic tunnels. Every test here runs under the exact K7 policy, with the browser's own violation report as a
// second channel (WP-44 T5), so a session never needs more than K7 allows (SEC-001).

let policy: PolicyReports;

test.use({ signedIn: true });
test.beforeEach(async ({ context }) => {
  policy = await servePolicy(context, k7Policy());
});
test.afterEach(({ watch }) => {
  expect(violationsSoFar(watch, policy)).toEqual({ events: [], reports: [] });
});

test('RA-007, D-5: an SSH session opens over the terminal tunnel and shows what the shell sent, under K7 with no violation', async ({
  page,
  hub,
  watch,
}) => {
  await openDevices(page);
  await page.getByRole('button', { name: 'SSH to gpu-box' }).click();

  // The tab title is the shell's own window title, set by bytes that came over the tunnel and went through xterm.js.
  const tab = page.getByRole('tab', { name: /gpu-box/ });
  await expect(tab).toContainText(TERMINAL_TITLE);
  await expect(page.getByRole('group', { name: 'Terminal of gpu-box' }).locator('.xterm')).toBeVisible();
  expect(hub.recording.where({ channel: 'terminal' }).map((found) => found.operation)).toEqual(['ticket accepted']);
  expect(hub.recording.where({ operation: 'StartSession' })).toHaveLength(1);
  expect(watch.pageErrors).toEqual([]);
  await expectNoViolations(page, watch, policy);
});

test('RA-001, D-5: a VNC session opens over the Guacamole tunnel and draws the remote screen, under K7 with no violation', async ({
  page,
  hub,
  watch,
}) => {
  await openDevices(page);
  await page.getByRole('button', { name: 'VNC to mac-mini' }).click();
  await expect(page.getByRole('tab', { name: /mac-mini/ })).toBeVisible();
  await expect.poll(() => hub.recording.where({ channel: 'tunnel', operation: 'ticket accepted' }).length).toBe(1);

  // The synthetic hub fills the screen with one colour (`cfill`); a canvas holding it shows the drawing reached the page.
  const filled = page.locator('canvas');
  await expect
    .poll(() =>
      filled.evaluateAll((canvases) =>
        canvases.some((canvas) => {
          const pixel = (canvas as HTMLCanvasElement).getContext('2d')?.getImageData(5, 5, 1, 1).data;
          return pixel !== undefined && pixel[0] === 32 && pixel[1] === 64 && pixel[2] === 96;
        }),
      ),
    )
    .toBe(true);
  expect(watch.pageErrors).toEqual([]);
  await expectNoViolations(page, watch, policy);
});

test('RA-002, CLI-001: a session tab offers no Pop out', async ({ page }) => {
  await openDevices(page);
  await page.getByRole('button', { name: 'SSH to gpu-box' }).click();
  await expect(page.getByRole('tab', { name: /gpu-box/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^More for / })).toHaveCount(0);
  await expect(page.getByText('Pop out')).toHaveCount(0);
});
