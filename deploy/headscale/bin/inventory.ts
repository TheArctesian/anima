// Usage: bun deploy/headscale/bin/inventory.ts nodes.json routes.json expected.txt
// Checks `headscale nodes list -o json` against config/policy.hujson (VPN-013). Exit 1 on a finding.
import { parseHujson } from '../src/hujson.ts';
import { findings, parseNodes, parseRoutes, type Finding } from '../src/inventory.ts';
import { parsePolicy } from '../src/policy.ts';

const [nodesPath, routesPath, expectedPath] = process.argv.slice(2);
if (nodesPath === undefined || routesPath === undefined || expectedPath === undefined) {
  process.stderr.write('usage: bun deploy/headscale/bin/inventory.ts nodes.json routes.json expected.txt\n');
  process.exit(2);
}

const policyUrl = new URL('../config/policy.hujson', import.meta.url);
const policy = parsePolicy(parseHujson(await Bun.file(policyUrl).text()));
const tailnet = { nodes: parseNodes(await Bun.file(nodesPath).json()), routes: parseRoutes(await Bun.file(routesPath).json()) };
const expected = (await Bun.file(expectedPath).text())
  .split('\n')
  .map((line) => line.trim())
  .filter((line) => line !== '');

function line(finding: Finding): string {
  const { kind, ...rest } = finding;
  const [subject, ...detail] = Object.values(rest);
  return `! ${kind} ${String(subject)}${detail.length > 0 ? ` (${detail.map(String).join(', ')})` : ''}`;
}

const found = findings(policy, tailnet, expected, new Date());
process.stdout.write(found.length === 0 ? 'inventory: ok\n' : `${found.map(line).join('\n')}\n`);
process.exit(found.length === 0 ? 0 : 1);
