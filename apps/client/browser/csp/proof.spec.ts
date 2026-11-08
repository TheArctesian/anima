import type { Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';
import { inlineScriptHashes } from '../synthetic/inline-scripts';
import { builtIndex, k7Policy } from './k7-policy';
import { expectNoViolations, servePolicy, violationsSeen, type PolicyReports } from './violations';

// The clean runs in csp.spec.ts prove something only if the same check fails when the page breaks the policy. Each case
// here breaks it once, on purpose, and shows both channels see it and `expectNoViolations` fails. Test-only: the header
// variant lives in this spec, never in the hub or the build.

test.use({ signedIn: true });

async function openDevices(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/devices`);
  await expect(page.getByText('mac-mini', { exact: true }).first()).toBeVisible();
}

test('SEC-001: without the start script hash the app never boots, and both channels report it from the first script', async ({ page, context, hub, watch }) => {
  const hashes = inlineScriptHashes(builtIndex());
  expect(hashes).toHaveLength(1);
  const policy = await servePolicy(context, k7Policy([]));
  const requested: string[] = [];
  page.on('request', (request) => requested.push(new URL(request.url()).pathname));
  await page.goto(`${HUB_ORIGIN}/devices`);
  const seen = await violationsSeen(page, watch, policy);
  // The enforced policy refused it, not only the Report-Only copy.
  const enforced = seen.events.filter((found) => found.disposition === 'enforce').map((found) => found.effectiveDirective);
  expect(enforced).toContain('script-src-elem');
  expect(seen.reports.map((found) => found.effectiveDirective)).toContain('script-src-elem');
  // index.html preloads the layout node (and some engines fetch what it imports) whether or not the start script runs. A
  // page's own route node is imported only by the running app, so no route node beyond the preloads means it never started.
  const preloaded = [...builtIndex().matchAll(/<link href="([^"]+)" rel="modulepreload">/g)].map((found) => found[1]);
  expect(preloaded.filter((path) => path?.startsWith('/_app/immutable/nodes/'))).not.toHaveLength(0);
  expect(requested.filter((path) => path.startsWith('/_app/immutable/nodes/') && !preloaded.includes(path))).toEqual([]);
  expect(requested).toContain('/devices');
  expect(hub.recording.requests).toEqual([]);
  await expect(expectNoViolations(page, watch, policy)).rejects.toThrow();
});

/** One thing the K7 policy forbids, done from inside the page after the app booted. */
type Breach =
  | 'inline script'
  | 'foreign script'
  | 'foreign stylesheet'
  | 'eval'
  | 'new Function'
  | 'string timer'
  | 'inline handler'
  | 'blob worker'
  | 'blob frame';

/** The directive each breach must be reported under; a browser may name the narrower or the fallback directive. */
const BREACHES: Record<Breach, readonly string[]> = {
  'inline script': ['script-src-elem', 'script-src'],
  'foreign script': ['script-src-elem', 'script-src'],
  'foreign stylesheet': ['style-src-elem', 'style-src'],
  eval: ['script-src'],
  'new Function': ['script-src'],
  'string timer': ['script-src'],
  'inline handler': ['script-src-attr', 'script-src'],
  'blob worker': ['worker-src', 'script-src', 'child-src'],
  'blob frame': ['frame-src', 'child-src', 'default-src'],
};

// Runs in the page. Every branch is something the build must never do; the policy must refuse it.
function breach(kind: Breach): void {
  const add = (element: HTMLElement): void => {
    document.body.append(element);
  };
  const blob = (text: string, type: string): string => URL.createObjectURL(new Blob([text], { type }));
  const attempt = (act: () => unknown): void => {
    try {
      act();
    } catch {
      // A refused eval or worker throws; the violation is what the spec reads.
    }
  };
  // Code run by the test driver itself may eval freely (DevTools lifts the check); a task the page queues may not.
  const later = (act: () => unknown): void => {
    setTimeout(() => {
      attempt(act);
    }, 0);
  };
  const script = document.createElement('script');
  const element = document.createElement('div');
  const frame = document.createElement('iframe');
  const sheet = document.createElement('link');
  switch (kind) {
    case 'inline script':
      script.textContent = 'window.animaBreach = 1;';
      add(script);
      break;
    case 'foreign script':
      script.src = 'https://example.net/breach.js';
      add(script);
      break;
    case 'foreign stylesheet':
      sheet.rel = 'stylesheet';
      sheet.href = 'https://example.net/breach.css';
      add(sheet);
      break;
    case 'eval':
      // The breach under test: the policy must refuse eval.
      later(() => eval('1'));
      break;
    case 'new Function':
      // The breach under test: the policy must refuse the Function constructor.
      later(() => new Function('return 1'));
      break;
    case 'string timer':
      // The breach under test: a string timer is an eval.
      setTimeout('window.animaBreach = 1;', 0);
      break;
    case 'inline handler':
      element.setAttribute('onclick', 'window.animaBreach = 1;');
      add(element);
      element.click();
      break;
    case 'blob worker':
      attempt(() => new Worker(blob('postMessage(1);', 'text/javascript')));
      break;
    case 'blob frame':
      frame.src = blob('<p>frame</p>', 'text/html');
      add(frame);
      break;
  }
}

test.describe('under the exact K7 policy', () => {
  let policy: PolicyReports;

  test.beforeEach(async ({ context }) => {
    policy = await servePolicy(context, k7Policy());
  });

  for (const [kind, directives] of Object.entries(BREACHES) as [Breach, readonly string[]][]) {
    test(`SEC-001: an injected ${kind} in the booted app is reported and fails the clean check`, async ({ page, hub, watch, browserName }) => {
      await openDevices(page);
      const named = (found: { effectiveDirective: string }): boolean => directives.includes(found.effectiveDirective);
      await page.evaluate(breach, kind);
      // A frame or a queued task is refused after `evaluate` returns.
      await expect.poll(() => watch.cspViolations.filter(named).length).toBeGreaterThan(0);
      const seen = await violationsSeen(page, watch, policy);
      expect(seen.events.filter(named), JSON.stringify(seen.events)).not.toHaveLength(0);
      // Firefox's frame-src report never reaches Playwright (seen with 1.65); its violation event above still does.
      const reportVisible = !(kind === 'blob frame' && browserName === 'firefox');
      if (reportVisible) expect(seen.reports.filter(named), JSON.stringify(seen.reports)).not.toHaveLength(0);
      await expect(expectNoViolations(page, watch, policy)).rejects.toThrow();
      // The policy stopped it in the page: nothing reached another origin.
      expect(hub.recording.foreignRequests).toBe(0);
    });
  }
});
