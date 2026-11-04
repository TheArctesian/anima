// config/render.sh refuses unsafe input and never writes a partial config
// (WP-03 T2; VPS-002, VPS-003). Runs the script with fake `ip` and `envsubst`.
import { afterAll, describe, expect, test } from 'bun:test';
import { chmodSync, existsSync, mkdtempSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const script = new URL('../config/render.sh', import.meta.url).pathname;
const templatePath = new URL('../config/config.yaml', import.meta.url).pathname;
const template = await Bun.file(templatePath).text();

const address = '80.240.10.10';
const email = 'operator@example.org';
const work = mkdtempSync(join(tmpdir(), 'anima-render-'));
afterAll(() => {
  rmSync(work, { recursive: true, force: true });
});

function executable(name: string, body: string): string {
  const path = join(work, name);
  writeFileSync(path, body);
  chmodSync(path, 0o755);
  return path;
}

// Like envsubst limited to the variables named in its argument.
const fakeEnvsubst = executable(
  'envsubst',
  `#!/usr/bin/env bun
const names = [...(process.argv[2] ?? '').matchAll(/\\$\\{(\\w+)\\}/g)].map((m) => m[1]);
const text = await Bun.stdin.text();
process.stdout.write(text.replace(/\\$\\{(\\w+)\\}/g, (whole, name) =>
  names.includes(name) ? (process.env[name] ?? '') : whole));
`,
);
const brokenEnvsubst = executable('envsubst-noop', '#!/bin/sh\nexec cat\n');
const ipWith = (shown: string): string =>
  executable(`ip-${shown}`, `#!/bin/sh\necho "2: eth0    inet ${shown}/24 brd 80.240.10.255 scope global eth0"\n`);
const ipOnVps = ipWith(address);

interface Run {
  ip?: string;
  email?: string;
  envsubst?: string;
  ipCommand?: string;
  template?: string;
}

let runs = 0;
function render(run: Run): { code: number; stderr: string; out: string } {
  runs += 1;
  const out = join(work, `config-${String(runs)}.yaml`);
  const env: Record<string, string> = {
    PATH: process.env['PATH'] ?? '',
    ENVSUBST: run.envsubst ?? fakeEnvsubst,
    IP: run.ipCommand ?? ipOnVps,
  };
  if (run.ip !== undefined) {
    env['VPS_PUBLIC_IPV4'] = run.ip;
  }
  env['ACME_EMAIL'] = run.email ?? email;
  const result = Bun.spawnSync(['bash', script, run.template ?? templatePath, out], { env });
  return { code: result.exitCode, stderr: result.stderr.toString(), out };
}

function expectRefused(run: Run, message: RegExp): void {
  const result = render(run);
  expect(result.code).not.toBe(0);
  expect(result.stderr).toMatch(message);
  expect(existsSync(result.out)).toBe(false);
}

describe('render.sh refuses unsafe input and writes nothing (VPS-002, VPS-003)', () => {
  test('unset, empty or malformed VPS_PUBLIC_IPV4 is refused', () => {
    for (const ip of [undefined, '', '1.2.3', '1.2.3.4.5', '01.2.3.4', '256.1.1.1', ' 80.240.10.10', '1.2.3.4\n']) {
      expectRefused({ ip }, /not a dotted-quad/);
    }
  });

  test('private, CGNAT, loopback, link-local and documentation addresses are refused', () => {
    for (const ip of ['10.0.0.1', '172.16.0.1', '192.168.1.1', '100.64.0.1', '127.0.0.1', '169.254.1.1', '203.0.113.10', '0.0.0.0']) {
      expectRefused({ ip }, /not a public address/);
    }
  });

  test('an address that is not on a local interface is refused', () => {
    expectRefused({ ip: '80.240.10.1', ipCommand: ipWith('80.240.10.11') }, /not on a local interface/);
  });

  test('an ACME_EMAIL that does not look like an address is refused', () => {
    for (const bad of ['', 'operator', 'a@b', 'a b@example.org', 'a"@example.org']) {
      expectRefused({ ip: address, email: bad }, /ACME_EMAIL/);
    }
  });

  test('a rendered config whose listeners are not on the VPS address is refused', () => {
    const wildcard = join(work, 'wildcard.yaml');
    writeFileSync(wildcard, template.replace(/^listen_addr: .*$/m, 'listen_addr: 0.0.0.0:443'));
    expectRefused({ ip: address, template: wildcard }, /listen_addr is not/);
  });

  test('a placeholder left by envsubst is refused', () => {
    expectRefused({ ip: address, envsubst: brokenEnvsubst }, /listen_addr is not|placeholder/);
  });
});

describe('render.sh output (VPS-002, VPS-003)', () => {
  test('good input writes a private config equal to the tested template rendering', async () => {
    const result = render({ ip: address });
    expect(result.code).toBe(0);
    const text = await Bun.file(result.out).text();
    const expected = template.replaceAll('${VPS_PUBLIC_IPV4}', address).replaceAll('${ACME_EMAIL}', email);
    expect(text).toBe(expected);
    const config = Bun.YAML.parse(text) as { listen_addr: string; derp: { server: { stun_listen_addr: string } } };
    expect(config.listen_addr).toBe(`${address}:443`);
    expect(config.derp.server.stun_listen_addr).toBe(`${address}:3478`);
    expect(statSync(result.out).mode & 0o777).toBe(0o600);
  });

  test('shellcheck is clean (a missing shellcheck is a failure)', () => {
    const shellcheck = Bun.which('shellcheck');
    expect(shellcheck).not.toBeNull();
    const result = Bun.spawnSync([shellcheck ?? 'shellcheck', script]);
    expect(result.stdout.toString()).toBe('');
    expect(result.exitCode).toBe(0);
  });
});
