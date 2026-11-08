import { test as base } from '@playwright/test';
import { installSyntheticHub, type SyntheticHub } from './synthetic/synthetic-hub';
import { watchPage, type PageWatch } from './page-watch';

interface Fixtures {
  /** Start the context signed in (a `__Host-anima` cookie) or signed out. Set per file with `test.use`. */
  signedIn: boolean;
  hub: SyntheticHub;
  watch: PageWatch;
}

/** `test` with the synthetic hub on every context and every page watched. Specs import it instead of Playwright's. */
export const test = base.extend<Fixtures>({
  signedIn: [false, { option: true }],
  hub: [
    async ({ context, signedIn }, use) => {
      const hub = await installSyntheticHub(context);
      if (signedIn) await hub.signIn();
      await use(hub);
    },
    { auto: true },
  ],
  watch: [
    async ({ page, hub }, use) => {
      void hub;
      await use(await watchPage(page));
    },
    { auto: true },
  ],
});

export { expect } from '@playwright/test';
