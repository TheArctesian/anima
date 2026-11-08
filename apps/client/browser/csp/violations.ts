import { expect, type BrowserContext, type Page, type Request } from '@playwright/test';
import { withoutQuery, type CspViolation, type PageWatch } from '../page-watch';
import { HUB_ORIGIN } from '../synthetic/access';
import { untilClosed } from '../synthetic/closed';
import { staticSite } from '../synthetic/static-files';

// The second channel for CSP violations: the browser's own report, sent by a Report-Only copy of the enforced policy. It
// needs no script in the page, so a violation during boot, before any page script runs, is reported here too.
// Firefox sends every CSP report past `context.route`, straight to the network, the clean sentinel reports too. The
// control is the `.invalid` report host (RFC 6761): it never resolves, so a report goes no further than a DNS lookup and
// no TLS connection or body is ever sent. A same-origin report URL would reach the real hub.anima.ing.

const REPORT_HOST = 'csp-report.invalid';
/** A URL every K7 `connect-src` refuses; fetching it is the end-of-test marker in both channels. */
const SENTINEL = 'https://csp-sentinel.invalid/';

/** What the browser reported about violations of the page's policy. */
export interface PolicyReports {
  readonly reports: readonly CspViolation[];
}

/** What the two channels saw, without the sentinel. */
export interface Seen {
  readonly events: readonly CspViolation[];
  readonly reports: readonly CspViolation[];
}

/** The rule and what it refused, from a CSP report body. Never the document URL or the script sample. */
function reportOf(request: Request): CspViolation {
  let body: unknown = null;
  try {
    body = request.postDataJSON();
  } catch {
    // A body that is not JSON is still a report; its fields stay unknown.
  }
  const report = ((body as { 'csp-report'?: Record<string, unknown> } | null)?.['csp-report'] ?? {}) as Record<string, unknown>;
  const field = (name: string): string => (typeof report[name] === 'string' ? report[name] : '');
  return {
    effectiveDirective: field('effective-directive') || field('violated-directive') || 'unknown',
    // Only the Report-Only copy names a report URL, so every report is of that copy.
    disposition: 'report',
    blockedUri: withoutQuery(field('blocked-uri')),
    sourceFile: withoutQuery(field('source-file')),
  };
}

/**
 * Serves every page of the hub origin with `policy` as its enforced CSP (the rest of the K7 headers unchanged), plus the
 * same policy as Report-Only with a report URL, and collects the reports. Call before the first navigation.
 */
export async function servePolicy(context: BrowserContext, policy: string): Promise<PolicyReports> {
  const site = staticSite();
  const reports: CspViolation[] = [];
  context.on('request', (request) => {
    if (new URL(request.url()).hostname === REPORT_HOST) reports.push(reportOf(request));
  });
  await context.route('**/*', (route) =>
    untilClosed(async () => {
      const request = route.request();
      const url = new URL(request.url());
      if (url.hostname === REPORT_HOST) {
        await route.fulfill({ status: 204, body: '' });
      } else if (request.resourceType() === 'document' && url.origin === HUB_ORIGIN) {
        const answer = site.answer(url.pathname);
        const reportOnly = `${policy}; report-uri https://${REPORT_HOST}/`;
        const headers = {
          ...answer.headers,
          'content-security-policy': policy,
          'content-security-policy-report-only': reportOnly,
        };
        await route.fulfill({
          status: answer.status,
          headers,
          body: answer.body,
        });
      } else {
        await route.fallback();
      }
    }, context),
  );
  return { reports };
}

const isSentinel = (violation: CspViolation): boolean => violation.blockedUri.includes('csp-sentinel.invalid');

/**
 * The violations both channels saw so far. It first makes one sentinel violation and waits until both channels show it,
 * so a dead channel, or a report still in flight, cannot pass for "none".
 */
export async function violationsSeen(page: Page, watch: PageWatch, policy: PolicyReports): Promise<Seen> {
  await page.evaluate(async (url) => {
    // eslint-disable-next-line no-restricted-globals -- test code in the page: the sentinel request the policy must refuse
    await fetch(url).catch(() => undefined);
  }, SENTINEL);
  await expect
    .poll(() => ({
      event: watch.cspViolations.some(isSentinel),
      report: policy.reports.some(isSentinel),
    }))
    .toEqual({ event: true, report: true });
  return violationsSoFar(watch, policy);
}

/** The violations in both channels, without the sentinel, with no new round trip: for a check after the test body. */
export function violationsSoFar(watch: PageWatch, policy: PolicyReports): Seen {
  const real = (violations: readonly CspViolation[]): CspViolation[] => violations.filter((violation) => !isSentinel(violation));
  return { events: real(watch.cspViolations), reports: real(policy.reports) };
}

/** Fails unless neither channel saw a violation since the page's first script. */
export async function expectNoViolations(page: Page, watch: PageWatch, policy: PolicyReports): Promise<void> {
  expect(await violationsSeen(page, watch, policy)).toEqual({
    events: [],
    reports: [],
  });
}
