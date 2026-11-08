import type { Page } from '@playwright/test';

/** One `securitypolicyviolation` event: which rule refused what. Never the page's text. */
export interface CspViolation {
  readonly effectiveDirective: string;
  readonly blockedUri: string;
  readonly sourceFile: string;
}

/** What went wrong in a page, collected from its start. */
export interface PageWatch {
  readonly cspViolations: readonly CspViolation[];
  readonly consoleErrors: readonly string[];
  readonly pageErrors: readonly string[];
}

const BINDING = 'animaReportViolation';

// Runs in the page before any of its scripts.
function listenForViolations(binding: string): void {
  document.addEventListener(
    'securitypolicyviolation',
    (event) => {
      const report = (globalThis as unknown as Record<string, (violation: CspViolation) => void>)[binding];
      report?.({ effectiveDirective: event.effectiveDirective, blockedUri: event.blockedURI, sourceFile: event.sourceFile });
    },
    true,
  );
}

/** Starts watching `page`; call before the first navigation. */
export async function watchPage(page: Page): Promise<PageWatch> {
  const cspViolations: CspViolation[] = [];
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  await page.exposeFunction(BINDING, (violation: CspViolation) => cspViolations.push(violation));
  await page.addInitScript(listenForViolations, BINDING);
  page.on('console', (message) => {
    if (message.type() === 'error') consoleErrors.push(message.text());
  });
  page.on('pageerror', (error) => pageErrors.push(error.message));
  return { cspViolations, consoleErrors, pageErrors };
}
