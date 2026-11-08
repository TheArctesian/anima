import { expect, test } from '@playwright/test';
import { escapedAttempts } from './escapes';
import type { Attempt } from './deny-proxy';

// Ruling S3: no test browser reaches a real host. The wall is a deny-all proxy under every engine (isolation/deny-proxy.ts);
// `global-teardown.ts` fails the run when an engine's proxy holds an attempt the harness does not explain. These specs prove
// the wall stands in each engine, and that the check reads it.

const PROBE = 'isolation-probe.invalid';

/** What this engine's proxy has been asked for so far; the config gave it to the project, the spec asks it by address. */
async function attemptsOf(address: string): Promise<readonly Attempt[]> {
  // eslint-disable-next-line no-restricted-globals -- test code in Node: it reads the proxy's own record, not the hub
  const response = await fetch(`${address}/attempts`);
  return (await response.json()) as Attempt[];
}

test('S3: a connection the browser makes by itself reaches this engine\'s deny-all proxy, is refused, and goes no further', async ({ browser }, testInfo) => {
  const address = process.env[`ANIMA_DENY_PROXY_${testInfo.project.name.toUpperCase()}`];
  expect(address, 'the config starts one proxy per engine').toBeDefined();
  // A context with no synthetic hub: nothing answers for the page, so the request leaves the browser for real.
  const context = await browser.newContext();
  const page = await context.newPage();
  await expect(page.goto(`https://${PROBE}/`)).rejects.toThrow();
  // A refused plain request may show as an error page or as a failed load, by engine; either way it got no further.
  const refused = await page.goto(`http://${PROBE}/`).catch(() => null);
  expect(refused?.ok() ?? false).toBe(false);
  await context.close();

  await expect
    .poll(async () => (await attemptsOf(address ?? '')).map((attempt) => `${attempt.via} ${attempt.target}`))
    .toEqual(expect.arrayContaining([`CONNECT ${PROBE}:443`, `HTTP ${PROBE}:80`]));
});

test.describe('escapedAttempts, the check at the end of the run', () => {
  const connect = (target: string): Attempt => ({ via: 'CONNECT', target });

  test('S3: an attempt at a real host, or at any port of the hub host in an engine that does not preconnect, is an escape', () => {
    const real = connect('example.net:443');
    expect(escapedAttempts('chromium', [real, connect('hub.anima.ing:443'), connect('csp-report.invalid:443')])).toEqual([
      real,
      connect('hub.anima.ing:443'),
    ]);
    expect(escapedAttempts('firefox', [connect('hub.anima.ing:443'), { via: 'HTTP', target: 'hub.anima.ing:80' }])).toHaveLength(2);
  });

  test('S3: WebKit\'s refused preconnect to the hub origin is explained; any other WebKit attempt is not', () => {
    const stray = connect('100.64.0.1:443');
    expect(escapedAttempts('webkit', [connect('hub.anima.ing:443'), stray, connect('hub.anima.ing:8443')])).toEqual([
      stray,
      connect('hub.anima.ing:8443'),
    ]);
  });

  test('S3: a fixture device host under annex.example is explained in every engine, and no other `.example` host is', () => {
    const fixture = connect('nas.annex.example:5001');
    for (const engine of ['chromium', 'webkit', 'firefox']) {
      expect(escapedAttempts(engine, [fixture, connect('nas.annex.example.net:443'), connect('other.example:443')])).toEqual([
        connect('nas.annex.example.net:443'),
        connect('other.example:443'),
      ]);
    }
  });

  test('S3: the Firefox CSP report host and the probe are the only `.invalid` hosts the harness asks for', () => {
    expect(escapedAttempts('firefox', [connect('csp-report.invalid:443'), connect('other.invalid:443')])).toEqual([connect('other.invalid:443')]);
  });
});
