import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables, type Chain } from "../src/nftables.ts";
import { evaluateChain, resolvedChain, type Packet, type Verdict } from "./nft-eval.ts";

// NOT-002, D-19, NET-001, WP-52: Apprise (172.30.3.10 on the notify bridge) fetches any URL a
// service URL parameter names. The host firewall keeps it off this host and every private,
// VPN, loopback, link-local, Docker and multicast range, and lets it reach the internet.
const ruleset = parseNftables(readFileSync(join(import.meta.dir, "..", "vps", "nftables.conf"), "utf8"));
const chain = (name: string): Chain => resolvedChain(ruleset, "inet anima_filter", name);

const APPRISE = "172.30.3.10";
const HUB = "172.30.3.2";
const NOTIFY_BRIDGE = "br-3f9a0c1d2e4b";
const PUBLIC_ADDRESS = "203.0.113.7";

/** A packet for this host: raw prerouting, then input if prerouting let it through. */
function delivered(packet: Packet): Verdict {
  if (evaluateChain(chain("prerouting"), packet) === "drop") return "drop";
  return evaluateChain(chain("input"), packet);
}

/** A packet routed through this host: raw prerouting, then forward. */
function forwarded(packet: Packet): Verdict {
  if (evaluateChain(chain("prerouting"), packet) === "drop") return "drop";
  return evaluateChain(chain("forward"), packet);
}

type Flow = Pick<Packet, "daddr" | "protocol" | "dport"> & Partial<Packet>;

const toHost = (saddr: string, flow: Flow): Packet =>
  ({ iifname: NOTIFY_BRIDGE, saddr, daddrIsLocal: true, ctState: "new", ...flow });

const onward = (saddr: string, flow: Flow): Packet =>
  ({ iifname: NOTIFY_BRIDGE, oifname: "eth0", saddr, daddrIsLocal: false, ctState: "new", ...flow });

const HOST_TARGETS: Flow[] = [
  { daddr: "172.30.3.1", protocol: "tcp", dport: 443 },
  { daddr: "172.30.3.1", protocol: "tcp", dport: 22 },
  { daddr: "172.30.3.1", protocol: "udp", dport: 3478 },
  { daddr: "172.30.3.1", protocol: "udp", dport: 41641 },
  { daddr: PUBLIC_ADDRESS, protocol: "tcp", dport: 443 },
  { daddr: PUBLIC_ADDRESS, protocol: "tcp", dport: 22 },
  { daddr: "100.64.0.1", protocol: "tcp", dport: 443 },
  { daddr: "172.30.0.1", protocol: "tcp", dport: 443 },
  { daddr: "172.17.0.1", protocol: "tcp", dport: 443 },
];

describe("Apprise and this host (NOT-002, D-19)", () => {
  for (const flow of HOST_TARGETS) {
    test(`NOT-002 Apprise to ${flow.daddr} ${String(flow.protocol)}/${String(flow.dport)} is dropped`, () => {
      expect(delivered(toHost(APPRISE, flow))).toBe("drop");
      expect(delivered(toHost(APPRISE, { ...flow, ctState: "established" }))).toBe("drop");
    });
  }

  test("VPS-002 the hub and other containers keep their access to the host", () => {
    expect(delivered(toHost(HUB, { daddr: "172.30.3.1", protocol: "tcp", dport: 443 }))).toBe("accept");
    expect(delivered(toHost("172.30.0.20", { daddr: "172.30.0.1", protocol: "tcp", dport: 443 }))).toBe("accept");
    expect(delivered(toHost(HUB, { daddr: "172.30.3.1", protocol: "tcp", dport: 5432, ctState: "established" }))).toBe("accept");
    expect(delivered(toHost(HUB, { daddr: "172.30.3.1", protocol: "tcp", dport: 22 }))).toBe("drop");
  });

  test("VPS-001 VPS-002 the internet and VPN rules are unchanged", () => {
    const internet = (dport: number): Packet =>
      ({ iifname: "eth0", saddr: "198.51.100.4", daddr: PUBLIC_ADDRESS, daddrIsLocal: true, ctState: "new", protocol: "tcp", dport });
    expect(delivered(internet(443))).toBe("accept");
    expect(delivered(internet(22))).toBe("drop");
    const vpn: Packet = { iifname: "tailscale0", saddr: "100.64.0.2", daddr: "100.64.0.1", daddrIsLocal: true, ctState: "new", protocol: "tcp", dport: 22 };
    expect(delivered(vpn)).toBe("accept");
  });
});

const DENIED = [
  "0.1.2.3", "10.10.0.5", "100.64.0.2", "100.100.100.100", "127.0.0.1", "169.254.169.254", "172.17.0.2",
  "172.30.0.10", "172.30.1.10", "172.31.255.254", "192.168.1.1", "224.0.0.251", "239.255.255.250", "240.0.0.1",
  "255.255.255.255",
];

describe("Apprise onward traffic (NOT-002, D-19, NET-001)", () => {
  for (const daddr of DENIED) {
    test(`NET-001 Apprise to ${daddr} is not forwarded`, () => {
      expect(forwarded(onward(APPRISE, { daddr, protocol: "tcp", dport: 80 }))).toBe("drop");
    });
  }

  test("NOT-002 Apprise sends nothing into the tailnet, even to a site subnet in a public range", () => {
    const toSite = onward(APPRISE, { daddr: "198.51.100.5", protocol: "tcp", dport: 443, oifname: "tailscale0" });
    expect(forwarded(toSite)).toBe("drop");
  });

  test.each([
    ["1.1.1.1", "tcp", 443],
    ["142.250.0.0", "tcp", 465],
    ["142.250.0.0", "tcp", 587],
    ["8.8.8.8", "udp", 53],
  ] as const)("NOT-002 Apprise reaches the internet: %s %s/%d", (daddr, protocol, dport) => {
    expect(forwarded(onward(APPRISE, { daddr, protocol, dport }))).toBe("accept");
  });

  // With br_netfilter, same-bridge traffic passes the forward hook with the bridge as both interfaces.
  const bridged = (saddr: string, daddr: string, ctState: Packet["ctState"]): Packet =>
    ({ iifname: NOTIFY_BRIDGE, oifname: NOTIFY_BRIDGE, saddr, daddr, daddrIsLocal: false, ctState, protocol: "tcp", dport: 8000 });

  test("D-19 the hub reaches Apprise and Apprise answers it", () => {
    expect(forwarded(bridged(HUB, APPRISE, "new"))).toBe("accept");
    expect(forwarded(bridged(APPRISE, HUB, "established"))).toBe("accept");
    expect(forwarded(bridged(APPRISE, HUB, "related"))).toBe("accept");
  });

  test("NOT-002 Apprise cannot open a connection to the hub or another address on notify", () => {
    expect(forwarded(bridged(APPRISE, HUB, "new"))).toBe("drop");
    expect(forwarded(bridged(APPRISE, "172.30.3.20", "new"))).toBe("drop");
  });

  test("R-050 other containers keep their forwarding", () => {
    expect(forwarded(onward(HUB, { daddr: "1.1.1.1", protocol: "tcp", dport: 443 }))).toBe("accept");
    expect(forwarded(onward(HUB, { daddr: "10.10.0.5", protocol: "tcp", dport: 5900, oifname: "tailscale0" }))).toBe("accept");
    expect(forwarded(onward("172.30.0.20", { daddr: "192.168.1.1", protocol: "tcp", dport: 3389, iifname: "br-1a2b3c4d5e6f" }))).toBe("accept");
  });
});

describe("Apprise address in the firewall and in compose.yaml (D-19)", () => {
  test("D-19 the firewall's Apprise address and network are the ones compose.yaml fixes", () => {
    const compose = readFileSync(join(import.meta.dir, "..", "..", "compose.yaml"), "utf8");
    const address = /^ {2}apprise:\n(?:^ {4}.*\n|^\s*\n)*?^ {6}notify:\n^ {8}ipv4_address: (\S+)$/m.exec(compose)?.[1];
    const subnet = /^ {2}notify:\n^ {4}ipam:\n^ {6}config:\n^ {8}- subnet: (\S+)$/m.exec(compose)?.[1];
    expect(ruleset.commands).toContain(`define apprise = ${String(address)}`);
    expect(ruleset.commands).toContain(`define apprise_network = ${String(subnet)}`);
  });
});

describe("packet evaluator forms used by the Apprise rules", () => {
  const packet: Packet = { iifname: "br-x", saddr: APPRISE, daddr: "10.0.0.1", daddrIsLocal: false, ctState: "new", protocol: "tcp", dport: 443 };
  const one = (rule: string): Chain => ({ table: "inet x", name: "c", rules: [rule], policy: "accept" });

  test("NOT-002 sets, negation, connection state and ports", () => {
    expect(evaluateChain(one("ip daddr { 192.168.0.0/16, 10.0.0.0/8 } drop"), packet)).toBe("drop");
    expect(evaluateChain(one("ip daddr != { 192.168.0.0/16, 10.0.0.0/8 } drop"), packet)).toBe("accept");
    expect(evaluateChain(one("ip saddr 172.30.3.10 ct state established,related drop"), packet)).toBe("accept");
    expect(evaluateChain(one("ct state != established drop"), packet)).toBe("drop");
    expect(evaluateChain(one("udp dport 443 drop"), packet)).toBe("accept");
    expect(evaluateChain(one("tcp dport 443 limit rate 5/second burst 10 packets drop"), packet)).toBe("drop");
  });

  test("NOT-002 the evaluator refuses a missing packet field and an unresolved define", () => {
    expect(() => evaluateChain(one("ct state new drop"), { ...packet, ctState: undefined })).toThrow("no ctState");
    expect(() => evaluateChain(one("ip saddr 10.0.0.1 drop"), { ...packet, saddr: undefined })).toThrow("no saddr");
    expect(() => evaluateChain(one("ip saddr $apprise drop"), packet)).toThrow("unresolved define");
    const bad = parseNftables("define a = 1.2.3.4\ntable inet x {\nchain c {\nip saddr $b drop\n}\n}\n");
    expect(() => resolvedChain(bad, "inet x", "c")).toThrow("undefined variable $b");
  });
});
