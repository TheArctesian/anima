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

const tunnelFile = 'apps/client/src/lib/sessions/tunnel/tunnel.ts';
const viewFile = 'apps/client/src/lib/sessions/view/display.ts';
const vendored = 'apps/client/src/lib/sessions/tunnel/vendor/guacamole.js';
const importFrom = (path: string) => `import Guacamole from '${path}';\nexport const g = Guacamole;\n`;

describe('ESLint Guacamole boundary (D-2, R-045)', () => {
  it('R-045: $lib/sessions/tunnel imports the vendored client', async () => {
    expect(await rulesFired(importFrom('./vendor/guacamole.js'), tunnelFile)).toEqual([]);
  });

  const vendorPaths = [
    '../tunnel/vendor/guacamole.js',
    '$lib/sessions/tunnel/vendor/guacamole.js',
    './tunnel/vendor',
    // Paths that do not spell `tunnel/vendor` but still reach the vendored file.
    '../tunnel/./vendor/guacamole.js',
    '$lib/sessions/tunnel//vendor/guacamole',
    '/src/lib/sessions/tunnel/vendor/guacamole.js?raw',
  ];
  for (const path of vendorPaths) {
    it(`D-2: ${path} is refused outside $lib/sessions/tunnel`, async () => {
      expect(await rulesFired(importFrom(path), viewFile)).toContain('no-restricted-imports');
    });
  }

  for (const path of ['../tunnel/vendor/guacamole.js', '../tunnel/./vendor/guacamole', 'guacamole-common-js']) {
    it(`D-2: a dynamic import() of ${path} is refused, with a string or a template literal`, async () => {
      expect(await rulesFired(`export const g = () => import('${path}');\n`, viewFile)).toContain('no-restricted-syntax');
      expect(await rulesFired(`export const g = () => import(\`${path}\`);\n`, viewFile)).toContain('no-restricted-syntax');
    });
  }

  it('D-2: the tunnel imports the client statically; a dynamic import is refused there too', async () => {
    expect(await rulesFired("export const g = () => import('./vendor/guacamole.js');\n", tunnelFile)).toContain('no-restricted-syntax');
  });

  it('D-2: other dynamic imports are allowed', async () => {
    expect(await rulesFired("export const g = () => import('./display');\n", viewFile)).toEqual([]);
  });

  it('R-045: the npm guacamole-common-js is refused everywhere, the tunnel included', async () => {
    expect(await rulesFired(importFrom('guacamole-common-js'), viewFile)).toContain('no-restricted-imports');
    expect(await rulesFired(importFrom('guacamole-common-js'), tunnelFile)).toContain('no-restricted-imports');
  });

  it('D-2: the vendored client is refused in $lib/hub and $lib/motion too', async () => {
    const path = '$lib/sessions/tunnel/vendor/guacamole.js';
    expect(await rulesFired(importFrom(path), 'apps/client/src/lib/hub/sample.ts')).toContain('no-restricted-imports');
    expect(await rulesFired(importFrom(path), 'apps/client/src/lib/motion/sample.ts')).toContain('no-restricted-imports');
  });

  it('D-2: the message names $lib/sessions/tunnel', async () => {
    const [result] = await eslint.lintText(importFrom('../tunnel/vendor/guacamole.js'), { filePath: `${root}${viewFile}` });
    expect(result?.messages[0]?.message).toContain('$lib/sessions/tunnel');
  });

  it('K4 §2: the tunnel may use WebSocket; motion and graphql-ws stay refused there', async () => {
    expect(await rulesFired("export const s = (url: string) => new WebSocket(url, 'guacamole');\n", tunnelFile)).toEqual([]);
    expect(await rulesFired("export const f = () => fetch('/x');\n", tunnelFile)).toContain('no-restricted-globals');
    expect(await rulesFired("import { animate } from 'motion';\nexport const a = animate;\n", tunnelFile)).toContain('no-restricted-imports');
  });

  it('R-045: the vendored file is not linted', async () => {
    expect(await eslint.isPathIgnored(`${root}${vendored}`)).toBe(true);
  });
});
