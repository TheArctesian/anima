import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables, type Chain } from "../src/nftables.ts";
import { evaluateChain, resolvedChain, type Packet } from "./nft-eval.ts";

// VP-13 security review F1: anima-probe, the hub's SSH user, is a shell on this host. The output
// chain keeps it off the host's own addresses, every container and the VPN; it may still reach
// the internet. Every other user is unaffected.
const VPS_DIR = join(import.meta.dir, "..", "vps");
const ruleset = parseNftables(readFileSync(join(VPS_DIR, "nftables.conf"), "utf8"));
const output: Chain = resolvedChain(ruleset, "inet anima_filter", "output");

const PROBE_UID = 10002;
const GATEWAY_BRIDGE = "br-5c0a77d1e2f3";

type Target = Pick<Packet, "daddr" | "oifname" | "daddrIsLocal" | "protocol" | "dport">;

const sentBy = (skuid: number, target: Target): Packet => ({ iifname: "", skuid, ...target });

const closed: [string, Target][] = [
  ["a port published on loopback", { daddr: "127.0.0.1", oifname: "lo", daddrIsLocal: true, protocol: "tcp", dport: 9090 }],
  ["the hub's published port on the VPN address", { daddr: "100.64.0.1", oifname: "lo", daddrIsLocal: true, protocol: "tcp", dport: 443 }],
  ["the public address", { daddr: "203.0.113.7", oifname: "lo", daddrIsLocal: true, protocol: "tcp", dport: 443 }],
  ["PostgreSQL's container", { daddr: "172.30.1.5", oifname: "br-0db0db0db0db", daddrIsLocal: false, protocol: "tcp", dport: 5432 }],
  ["the hub's container", { daddr: "172.30.0.10", oifname: GATEWAY_BRIDGE, daddrIsLocal: false, protocol: "tcp", dport: 8443 }],
  ["a container on docker0", { daddr: "172.17.0.2", oifname: "docker0", daddrIsLocal: false, protocol: "tcp", dport: 80 }],
  ["a tailnet node", { daddr: "100.64.0.2", oifname: "tailscale0", daddrIsLocal: false, protocol: "tcp", dport: 22 }],
  ["a site device", { daddr: "10.10.0.5", oifname: "tailscale0", daddrIsLocal: false, protocol: "tcp", dport: 3389 }],
  ["MagicDNS", { daddr: "100.100.100.100", oifname: "tailscale0", daddrIsLocal: false, protocol: "udp", dport: 53 }],
];

const internet: Target = { daddr: "198.51.100.4", oifname: "eth0", daddrIsLocal: false, protocol: "tcp", dport: 443 };

describe("anima-probe's outbound traffic (VP-13 F1)", () => {
  test.each(closed)("VP-13 anima-probe may not reach %s", (_, target) => {
    expect(evaluateChain(output, sentBy(PROBE_UID, target))).toBe("drop");
  });

  test("VP-13 anima-probe may reach the internet", () => {
    expect(evaluateChain(output, sentBy(PROBE_UID, internet))).toBe("accept");
  });

  test.each([
    ["root (sshd's replies to the hub, the host's own services)", 0],
    ["the admin user", 1000],
    ["anima-hub, the broker socket's user", 10001],
  ])("VP-13 the output chain leaves %s alone", (_, skuid) => {
    for (const [, target] of closed) expect(evaluateChain(output, sentBy(skuid, target))).toBe("accept");
  });

  test("VP-13 the output chain's uid is the one harden.sh probe gives anima-probe", () => {
    expect(ruleset.commands).toContain(`define probe_uid = ${String(PROBE_UID)}`);
    expect(readFileSync(join(VPS_DIR, "probe.sh"), "utf8")).toMatch(new RegExp(`^PROBE_ID=${String(PROBE_UID)}$`, "m"));
  });
});

describe("packet evaluator meta skuid", () => {
  const one = (rule: string): Chain => ({ table: "inet x", name: "c", rules: [rule], policy: "accept" });

  test("VP-13 meta skuid matches the socket's uid, in a set and negated, and needs it", () => {
    const packet = sentBy(PROBE_UID, internet);
    expect(evaluateChain(one("meta skuid 10002 drop"), packet)).toBe("drop");
    expect(evaluateChain(one("meta skuid { 0, 10002 } drop"), packet)).toBe("drop");
    expect(evaluateChain(one("meta skuid != 10002 drop"), packet)).toBe("accept");
    expect(evaluateChain(one("meta skuid 0 drop"), packet)).toBe("accept");
    expect(() => evaluateChain(one("meta skuid 0 drop"), { ...packet, skuid: undefined })).toThrow("no skuid");
  });
});
