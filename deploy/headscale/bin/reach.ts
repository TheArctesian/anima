// Usage: bun deploy/headscale/bin/reach.ts > /tmp/before.md   (then edit the policy, run again, diff)
// Prints the reachability table of config/policy.hujson for the fixture nodes (VPN-012).
import { nodes, policy } from '../tests/policy-fixture.ts';
import { reachMarkdown, reachTable } from '../src/reach.ts';

process.stdout.write(reachMarkdown(reachTable(policy, nodes)));
