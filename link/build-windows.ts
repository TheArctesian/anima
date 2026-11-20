// Native Windows link build; the pinned Tailcat module owns the required build tags.
import { spawnSync } from 'node:child_process';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

if (process.platform !== 'win32') throw new Error('This recipe builds on Windows.');
const cwd = import.meta.dir;
function go(args: string[], capture = false): string {
  const result = spawnSync('go.exe', args, {
    cwd, windowsHide: true, encoding: 'utf8',
    stdio: ['ignore', capture ? 'pipe' : 'inherit', 'inherit'],
    env: { ...process.env, GOOS: 'windows', GOARCH: 'amd64', CGO_ENABLED: '0' },
  });
  if (result.error !== undefined) throw result.error;
  if (result.status !== 0) throw new Error('The Windows link build failed.');
  return result.stdout ?? '';
}
go(['mod', 'download', 'github.com/tailscale/tailcat']);
const module = go(['list', '-m', '-f', '{{.Dir}}', 'github.com/tailscale/tailcat'], true).trim();
const tags = (await readFile(join(module, 'build-tags.txt'), 'utf8')).split(/\r?\n/).map(line => line.trim()).filter(Boolean);
if (!tags.includes('ts_omit_portmapper')) throw new Error('Tailcat must omit port mapping.');
await mkdir(join(cwd, 'bin'), { recursive: true });
go(['build', '-trimpath', '-ldflags=-s -w -H windowsgui', '-tags', tags.join(','), '-o', 'bin/anima-link-windows-amd64.exe', './cmd/anima-link']);
