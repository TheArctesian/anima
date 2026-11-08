import { fileURLToPath } from 'node:url';
import { ESLint } from 'eslint';
import { describe, expect, it, vi } from 'vitest';

// Linting loads the whole config; under a parallel workspace run it can pass 5 s.
vi.setConfig({ testTimeout: 30_000 });

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const eslint = new ESLint({ cwd: root });

async function rulesFired(code: string, filePath: string) {
  const [result] = await eslint.lintText(code, { filePath: `${root}${filePath}` });
  return result?.messages.map((m) => m.ruleId) ?? [];
}

const lib = 'apps/client/src/lib/devices/sample.ts';
const hubFile = 'apps/client/src/lib/hub/sample.ts';

describe('ESLint hub face (Q2-12, CQ1-17)', () => {
  const tsRule = '@typescript-eslint/no-restricted-imports';
  const use = (specifier: string) => `import { x } from '${specifier}';\nexport const y = x;\n`;
  const testFile = 'apps/client/src/lib/devices/sample.test.ts';
  const fixtureFile = 'apps/client/src/lib/devices/sample.fixture.ts';

  for (const internal of ['error', 'address', 'request', 'status/status', 'socket/socket', 'generation']) {
    it(`CQ1-17: $lib/hub/${internal} is refused outside lib/hub, in code and in tests`, async () => {
      expect(await rulesFired(use(`$lib/hub/${internal}`), lib)).toContain(tsRule);
      expect(await rulesFired(use(`$lib/hub/${internal}`), testFile)).toContain(tsRule);
    });
  }

  it('CQ1-17: a relative path into lib/hub internals is refused', async () => {
    expect(await rulesFired(use('../hub/error'), lib)).toContain(tsRule);
    expect(await rulesFired(use('./hub/error'), 'apps/client/src/lib/sample.ts')).toContain(tsRule);
    expect(await rulesFired("export { x } from '../hub/request';\n", lib)).toContain(tsRule);
  });

  for (const face of ['hub', 'context', 'generated', 'generated/graphql', 'notice/LiveNotice.svelte']) {
    it(`CQ1-17: the face $lib/hub/${face} is allowed outside lib/hub`, async () => {
      expect(await rulesFired(use(`$lib/hub/${face}`), lib)).not.toContain(tsRule);
      expect(await rulesFired(use(`../hub/${face}`), lib)).not.toContain(tsRule);
    });
  }

  it('CQ1-17: lib/hub files may import their own internals', async () => {
    expect(await rulesFired(use('./error'), hubFile)).not.toContain(tsRule);
    expect(await rulesFired(use('$lib/hub/error'), hubFile)).not.toContain(tsRule);
  });

  it('CQ1-17: the socket fake is allowed in tests and fixtures, refused in code', async () => {
    const fake = use('$lib/hub/socket/fake');
    expect(await rulesFired(fake, testFile)).not.toContain(tsRule);
    expect(await rulesFired(fake, fixtureFile)).not.toContain(tsRule);
    expect(await rulesFired(fake, lib)).toContain(tsRule);
  });

  it('CQ1-17: the codegen entry may import the generator', async () => {
    expect(await rulesFired(use('./src/lib/hub/generation.ts'), 'apps/client/codegen.ts')).not.toContain(tsRule);
  });

  it('CQ1-17: the restriction does not replace the older import bans', async () => {
    const tauri = "import { invoke } from '@tauri-apps/api/core';\nexport const i = invoke;\n";
    const both = `${tauri}${use('$lib/hub/error')}`;
    const fired = await rulesFired(both, lib);
    expect(fired).toContain('no-restricted-imports');
    expect(fired).toContain(tsRule);
    expect(await rulesFired(use('graphql-ws'), testFile)).not.toContain(tsRule);
  });
});

describe('ESLint updater boundary (UPD-001)', () => {
  const tsRule = '@typescript-eslint/no-restricted-imports';
  const desktopFile = 'apps/client/src/lib/desktop/sample.ts';
  const updatesFile = 'apps/client/src/lib/updates/sample.ts';

  it('UPD-001: a value import of the updater by $lib path is refused', async () => {
    const code = "import { desktopUpdater } from '$lib/desktop/updater';\nexport const u = desktopUpdater;\n";
    expect(await rulesFired(code, lib)).toContain(tsRule);
    expect(await rulesFired(code, updatesFile)).toContain(tsRule);
  });

  it('UPD-001: a relative value import of the updater is refused', async () => {
    const code = "import { desktopUpdater } from '../desktop/updater';\nexport const u = desktopUpdater;\n";
    expect(await rulesFired(code, lib)).toContain(tsRule);
  });

  it('UPD-001: ./updater inside lib/desktop is refused, as an import and as a re-export', async () => {
    const imported = "import { desktopUpdater } from './updater';\nexport const u = desktopUpdater;\n";
    expect(await rulesFired(imported, desktopFile)).toContain(tsRule);
    expect(await rulesFired("export { desktopUpdater } from './updater';\n", desktopFile)).toContain(tsRule);
    expect(await rulesFired("export * from './updater.js';\n", desktopFile)).toContain(tsRule);
  });

  it('UPD-001: a re-export of the updater from elsewhere is refused', async () => {
    expect(await rulesFired("export { desktopUpdater } from '$lib/desktop/updater';\n", lib)).toContain(tsRule);
  });

  it('UPD-001: a type import or type re-export of the updater is allowed', async () => {
    const type = "import type { UpdaterPort } from '$lib/desktop/updater';\nexport type U = UpdaterPort;\n";
    expect(await rulesFired(type, lib)).not.toContain(tsRule);
    expect(await rulesFired("export type { UpdaterPort } from './updater';\n", desktopFile)).not.toContain(tsRule);
    expect(await rulesFired("import { type UpdaterPort } from '../desktop/updater';\nexport type U = UpdaterPort;\n", lib)).not.toContain(tsRule);
  });

  it('UPD-001: the lazy dynamic import is allowed, and tests may import the updater', async () => {
    const lazy = "export const load = () => import('$lib/desktop/updater');\n";
    expect(await rulesFired(lazy, updatesFile)).not.toContain(tsRule);
    const value = "import { desktopUpdater } from '$lib/desktop/updater';\nexport const u = desktopUpdater;\n";
    expect(await rulesFired(value, 'apps/client/src/lib/updates/sample.test.ts')).not.toContain(tsRule);
  });

  it('UPD-001: another ./ import inside lib/desktop is allowed', async () => {
    expect(await rulesFired("import { a } from './thing';\nexport const b = a;\n", desktopFile)).not.toContain(tsRule);
  });

  it('CLI-003: @tauri-apps is still refused outside lib/desktop in a file the updater rule covers', async () => {
    const code = "import { invoke } from '@tauri-apps/api/core';\nexport const i = invoke;\n";
    expect(await rulesFired(code, updatesFile)).toContain('no-restricted-imports');
  });
});
