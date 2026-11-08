import { expect, test } from '../test-hub';
import { k7Policy } from '../csp/k7-policy';
import { openSignIn, sessionCookie, signInWith } from '../sign-in-steps';
import { callGraphql } from '../in-page';
import { COOKIE_MAX_AGE_SECONDS, HUB_ORIGIN, OPERATOR, SESSION_COOKIE } from '../synthetic/access';

// Plan criterion 4, client side and synthetic hub (SEC-002, API-003, D-3, D-4): what a browser does with the K7 cookie, who
// the hub lets it speak for, what Sign out leaves behind, and the headers on the static files.
//
// Not here, and not faked: the real hub's side of criterion 4. That `POST /graphql` and the `GET /graphql` upgrade with the
// cookie are refused when the request's `Origin` is another one, that `logout` deletes the token, that the real `Set-Cookie`
// and static headers are as K7 says, and that `CLIENT_ORIGINS` is unchanged belong to WP-43 (hub) and WP-53 (acceptance
// against the real hub). Playwright cannot show the upgrade's `Origin`, so no spec here can exercise a socket from another
// origin either; `synthetic/graphql/graphql-socket.ts` says so too.

const OTHER_ORIGIN = 'https://elsewhere.annex.example';
const ME = 'query Me { me { username } }';

test.describe('the cookie', () => {
  test('SEC-002, D-3: sign-in sets __Host-anima with Secure, HttpOnly, SameSite=Strict, Path=/ and no Domain, for seven days', async ({ page, context }) => {
    // Playwright answers the request for the hub, so the browser's own handling of `Set-Cookie` is not exercised: the hub puts the
    // cookie into the jar with these attributes, `pure.spec.ts` holds the `Set-Cookie` text to K7, and the real hub's text is WP-43's.
    await openSignIn(page);
    await signInWith(page, OPERATOR.password);
    await page.waitForURL(`${HUB_ORIGIN}/devices`);
    const [cookie, ...others] = await context.cookies(HUB_ORIGIN);
    expect(others).toEqual([]);
    expect(cookie).toMatchObject({ name: '__Host-anima', path: '/', httpOnly: true, secure: true, sameSite: 'Strict' });
    // A `__Host-` cookie may carry no Domain attribute; the browser reports the host it was set for.
    expect(cookie?.domain).toBe('hub.anima.ing');
    expect((cookie?.expires ?? 0) - Date.now() / 1000).toBeGreaterThan(COOKIE_MAX_AGE_SECONDS - 120);
    expect(await page.evaluate(() => document.cookie)).toBe('');
  });
});

test.describe('another origin', () => {
  test.use({ signedIn: true });

  test('SEC-002, API-003: a POST to /graphql from a page of another origin carries no cookie and is signed out', async ({
    page,
    context,
    hub,
    browserName,
  }) => {
    // The hub's own wall has no say over a page the test serves for another origin, so the test route answers it.
    await context.route(`${OTHER_ORIGIN}/**`, (route) => route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><p>another site</p>' }));
    const toHub: { cookie: string; origin: string }[] = [];
    await context.route(`${HUB_ORIGIN}/graphql`, async (route) => {
      const headers = await route.request().allHeaders();
      toHub.push({ cookie: headers.cookie ?? '', origin: headers.origin ?? '' });
      await route.fallback();
    });

    // From the hub's own page the cookie travels; the hub knows the operator.
    await page.goto(`${HUB_ORIGIN}/health`);
    expect((await callGraphql(page, ME)).data).toEqual({ me: { username: OPERATOR.username } });
    expect(toHub.at(-1)?.origin).toBe(HUB_ORIGIN);
    // WebKit does not show the `Cookie` header to a routed request, so only Chromium and Firefox prove the cookie travelled here.
    if (browserName !== 'webkit') expect(toHub.at(-1)?.cookie).toContain(SESSION_COOKIE);

    // From another site the browser sends its own `Origin`, and, SameSite=Strict, no cookie.
    await page.goto(`${OTHER_ORIGIN}/`);
    await page.evaluate(
      async ([target, body]) => {
        // eslint-disable-next-line no-restricted-globals -- test code in the page: it plays another site's request on purpose
        await fetch(target ?? '', { method: 'POST', mode: 'no-cors', credentials: 'include', body });
      },
      [`${HUB_ORIGIN}/graphql`, JSON.stringify({ query: ME })] as const,
    );
    expect(toHub.at(-1)).toEqual({ cookie: '', origin: OTHER_ORIGIN });
    const answered = hub.recording.where({ operation: 'Me' });
    expect(answered.map((found) => [found.origin, found.signedIn])).toEqual([
      [HUB_ORIGIN, true],
      [OTHER_ORIGIN, false],
    ]);
  });
});

test.describe('sign out', () => {
  test.use({ signedIn: true });

  test('SEC-002, D-3: Sign out clears the cookie, and the token it held no longer signs anyone in', async ({ page, context }) => {
    await page.goto(`${HUB_ORIGIN}/settings`, { waitUntil: 'domcontentloaded' });
    const oldToken = await sessionCookie(context);
    expect(oldToken).toBeDefined();
    await page.getByRole('button', { name: 'Sign out', exact: true }).click();
    await page.waitForURL(`${HUB_ORIGIN}/sign-in`);
    expect(await sessionCookie(context)).toBeUndefined();

    // The cookie is gone from the browser; the deletion on the hub is shown by putting the old token back.
    await context.addCookies([
      { name: SESSION_COOKIE, value: oldToken ?? '', url: HUB_ORIGIN, httpOnly: true, secure: true, sameSite: 'Strict' },
    ]);
    expect((await callGraphql(page, ME)).errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
  });
});

test.describe('the static files', () => {
  test.use({ signedIn: true });

  test('SEC-001, D-2: every file the hub serves to the page carries the K7 headers, and the page carries the K7 policy', async ({ page }) => {
    const served: { path: string; type: string; headers: Record<string, string> }[] = [];
    page.on('response', (response) => {
      const url = new URL(response.url());
      if (url.origin === HUB_ORIGIN && url.pathname !== '/graphql') {
        served.push({ path: url.pathname, type: response.request().resourceType(), headers: response.headers() });
      }
    });
    await page.goto(`${HUB_ORIGIN}/devices`);
    await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();

    expect(served.some((file) => file.type === 'document')).toBe(true);
    expect(served.some((file) => file.path.startsWith('/_app/immutable/') && file.type === 'script')).toBe(true);
    for (const { path, headers } of served) {
      expect(
        {
          nosniff: headers['x-content-type-options'],
          referrer: headers['referrer-policy'],
          opener: headers['cross-origin-opener-policy'],
          hsts: headers['strict-transport-security'],
          policy: headers['content-security-policy'],
        },
        path,
      ).toEqual({ nosniff: 'nosniff', referrer: 'no-referrer', opener: 'same-origin', hsts: 'max-age=31536000', policy: k7Policy() });
    }
    const cache = Object.fromEntries(served.map((file) => [file.path.startsWith('/_app/immutable/'), file.headers['cache-control']]));
    expect(cache).toEqual({ true: 'public, max-age=31536000, immutable', false: 'no-cache' });
  });
});
