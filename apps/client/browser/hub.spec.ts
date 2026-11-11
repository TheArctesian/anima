import type { Page } from '@playwright/test';
import { callGraphql, talkOnSocket } from './in-page';
import { expect, test } from './test-hub';
import { BEARER_TOKEN, HUB_ORIGIN, OPERATOR, SESSION_COOKIE } from './synthetic/access';

// The synthetic hub itself: authentication per K7, tickets per K4, and the wall around other origins. Later specs trust this.

const ME = 'query Me { me { username } }';
const SIGN_IN = 'mutation SignIn($input: LoginInput!) { signInBrowser(input: $input) { operator { username } expiresAt } }';
const LOGOUT = 'mutation Logout { logout }';
const START = 'mutation Start($input: StartSessionInput!) { startSession(input: $input) { sessionId token tunnelPath expiresAt } }';
const SSH_DEVICE = '0190a000-0000-7000-8012-000000000004';
const VNC_DEVICE = '0190a000-0000-7000-8012-000000000003';
const DISPLAY = { widthPx: 800, heightPx: 600 };

async function open(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/sign-in`);
}

test.describe('signed out', () => {
  test('API-002: a signed-out request is UNAUTHENTICATED except one login or signInBrowser', async ({ page, hub }) => {
    await open(page);
    const refused = await callGraphql(page, ME);
    expect(refused.errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
    const two = await callGraphql(page, 'mutation Two { logout login(input: {username: "x", password: "y"}) { token } }');
    expect(two.errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
    expect(hub.recording.where({ operation: 'Me' })[0]?.signedIn).toBe(false);
  });

  test('CLI-001: signInBrowser sets the __Host-anima cookie with its attributes and returns no token', async ({ page, context }) => {
    await open(page);
    const wrong = await callGraphql(page, SIGN_IN, { input: { username: OPERATOR.username, password: 'wrong', totpCode: OPERATOR.code } });
    expect(wrong.errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
    expect(await context.cookies(HUB_ORIGIN)).toEqual([]);

    const input = { username: OPERATOR.username, password: OPERATOR.password, totpCode: OPERATOR.code };
    const answer = await callGraphql(page, SIGN_IN, { input });
    expect(answer.data).toEqual({ signInBrowser: { operator: { username: 'ann' }, expiresAt: expect.any(String) as unknown } });
    const cookie = (await context.cookies(HUB_ORIGIN)).find((found) => found.name === SESSION_COOKIE);
    expect(cookie).toMatchObject({ httpOnly: true, secure: true, sameSite: 'Strict', path: '/' });
    expect(await page.evaluate(() => document.cookie)).toBe('');
    expect((await callGraphql(page, ME)).data).toEqual({ me: { username: 'ann' } });
  });

  test('API-002: a socket without the cookie is closed with 4403', async ({ page }) => {
    await open(page);
    const outcome = await talkOnSocket(page, 'wss://hub.anima.ing/graphql', 'graphql-transport-ws', {
      send: ['{"type":"connection_init","payload":{}}'],
      expect: 1,
    });
    expect(outcome).toEqual({ frames: [], closeCode: 4403 });
  });
});

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('API-002: the cookie signs a request in only without an Authorization header; logout clears it', async ({ page, context, hub }) => {
    await open(page);
    expect((await callGraphql(page, ME)).data).toEqual({ me: { username: 'ann' } });
    const withBadBearer = await callGraphql(page, ME, {}, { authorization: 'Bearer not-the-token' });
    expect(withBadBearer.errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
    expect((await callGraphql(page, ME, {}, { authorization: `Bearer ${BEARER_TOKEN}` })).data).toEqual({ me: { username: 'ann' } });
    expect(hub.recording.where({ operation: 'Me' }).map((found) => [found.hadAuthorization, found.hadCookie, found.signedIn])).toEqual([
      [false, true, true],
      [true, true, false],
      [true, true, true],
    ]);

    expect((await callGraphql(page, LOGOUT)).data).toEqual({ logout: true });
    expect(await context.cookies(HUB_ORIGIN)).toEqual([]);
    expect((await callGraphql(page, ME)).errors?.[0]?.extensions?.code).toBe('UNAUTHENTICATED');
  });

  test('API-002: a socket whose payload has a bad Authorization is closed even with the cookie', async ({ page }) => {
    await open(page);
    const outcome = await talkOnSocket(page, 'wss://hub.anima.ing/graphql', 'graphql-transport-ws', {
      send: ['{"type":"connection_init","payload":{"authorization":"Bearer wrong"}}'],
      expect: 1,
    });
    expect(outcome).toEqual({ frames: [], closeCode: 4403 });
  });

  test('API-002: a socket with the cookie is acknowledged', async ({ page }) => {
    await open(page);
    const outcome = await talkOnSocket(page, 'wss://hub.anima.ing/graphql', 'graphql-transport-ws', {
      send: ['{"type":"connection_init","payload":{}}'],
      expect: 1,
    });
    expect(outcome).toEqual({ frames: ['{"type":"connection_ack"}'], closeCode: null });
  });

});

test.describe('tunnels and the wall', () => {
  test.use({ signedIn: true });

  test('RA-007: an SSH ticket opens the terminal once, and shows a prompt after the first resize', async ({ page }) => {
    await open(page);
    const started = await callGraphql(page, START, { input: { deviceId: SSH_DEVICE, protocol: 'SSH', display: DISPLAY } });
    const ticket = (started.data as { startSession: { token: string; tunnelPath: string } }).startSession;
    expect(ticket.tunnelPath).toBe('/terminal');
    const url = `wss://hub.anima.ing${ticket.tunnelPath}?token=${ticket.token}`;
    const resize = '{"type":"resize","cols":80,"rows":24}';
    const first = await talkOnSocket(page, url, 'anima-terminal', { send: [resize], expect: 1 });
    expect(first.frames).toHaveLength(1);
    expect(first.frames[0]).toMatch(/^binary:\d+$/);
    const second = await talkOnSocket(page, url, 'anima-terminal', { send: [resize], expect: 1 });
    expect(second.closeCode).not.toBeNull();
  });

  test('RA-001: a VNC ticket opens the Guacamole tunnel with size, rect, cfill and sync', async ({ page }) => {
    await open(page);
    const started = await callGraphql(page, START, { input: { deviceId: VNC_DEVICE, protocol: 'VNC', display: DISPLAY } });
    const ticket = (started.data as { startSession: { token: string; tunnelPath: string } }).startSession;
    expect(ticket.tunnelPath).toBe('/tunnel');
    const outcome = await talkOnSocket(page, `wss://hub.anima.ing/tunnel?token=${ticket.token}`, 'guacamole', { send: [], expect: 5 });
    expect(outcome.frames.map((frame) => frame.split(',')[0])).toEqual(['0.', '4.size', '4.rect', '5.cfill', '4.sync']);
  });

  test('CON-003: a request to another origin is aborted and counted', async ({ page, hub }) => {
    await open(page);
    expect(hub.recording.foreignRequests).toBe(0);
    // The page's own CSP stops a script from reaching another origin; a navigation is what reaches the hub's wall.
    await expect(page.goto('https://elsewhere.annex.example/anything')).rejects.toThrow();
    expect(hub.recording.foreignOrigins).toEqual(['https://elsewhere.annex.example']);
  });
});
