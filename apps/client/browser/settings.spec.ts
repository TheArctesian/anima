import { expect, test } from './test-hub';

test.use({ signedIn: true });

test('PRJ-039: GitHub connection guidance and actions fit at narrow and medium widths', async ({ page }) => {
  await page.route('**/graphql', async (route) => {
    const request = route.request().postDataJSON() as { query: string };
    if (!request.query.startsWith('query GithubConnect ')) return route.fallback();
    await route.fulfill({ json: { data: { githubConnect: {
      id: 'waiting-attempt',
      phase: 'AT_GITHUB',
      startedBy: 'USER',
      expiresAt: '2026-11-13T12:55:00Z',
      pageUrl: 'https://hub.anima.ing/connect/github',
      appSlug: null,
      ownerLogin: null,
      installUrl: null,
      appSettingsUrl: null,
      failure: null,
      message: null,
      account: null,
    } } } });
  });
  await page.goto('/settings');
  const integrations = page.getByRole('region', { name: 'Integrations' });
  const hint = integrations.getByText('Makes a GitHub App in your account', { exact: false });
  const waiting = integrations.getByText('Waiting for you on GitHub', { exact: false });
  const actions = ['Continue in browser', 'Cancel', 'Use key file'];

  await expect(waiting).toBeVisible();
  await page.evaluate(() => document.fonts.ready.then(() => undefined));

  for (const width of [858, 320]) {
    await page.setViewportSize({ width, height: 600 });
    const sectionBox = await integrations.boundingBox();
    const hintBox = await hint.boundingBox();
    const waitingBox = await waiting.boundingBox();
    if (sectionBox === null || hintBox === null || waitingBox === null) throw new Error('The GitHub guidance is missing.');

    expect(hintBox.y + hintBox.height).toBeLessThanOrEqual(waitingBox.y);
    for (const name of actions) {
      const button = integrations.getByRole('button', { name, exact: true });
      const box = await button.boundingBox();
      if (box === null) throw new Error(`The ${name} button is missing.`);
      expect(box.y).toBeGreaterThanOrEqual(waitingBox.y + waitingBox.height);
      expect(box.x).toBeGreaterThanOrEqual(sectionBox.x);
      expect(box.x + box.width).toBeLessThanOrEqual(sectionBox.x + sectionBox.width);
    }
  }
});
