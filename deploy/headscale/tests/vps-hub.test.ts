import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseNftables, type Chain } from "../src/nftables.ts";
import { evaluateChain, resolvedChain, type Packet, type Verdict } from "./nft-eval.ts";

// VP-13, VPS-001, DNS-005: the hub container (172.30.0.10 on the gateway bridge, the network of
// its default route) may reach this host's VPN address 100.64.0.1 on 22/tcp and ICMP echo, so
// the VPS can be a device in Anima. Nothing else on a bridge reaches that address.
const ruleset = parseNftables(readFileSync(join(import.meta.dir, "..", "vps", "nftables.conf"), "utf8"));
const chain = (name: string): Chain => resolvedChain(ruleset, "inet anima_filter", name);

const HUB = "172.30.0.10";
const HUB_NOTIFY = "172.30.3.2";
const GUACD = "172.30.0.20";
const APPRISE = "172.30.3.10";
const VPN_ADDRESS = "100.64.0.1";
const PUBLIC_ADDRESS = "203.0.113.7";
const GATEWAY_BRIDGE = "br-5c0a77d1e2f3";
const NOTIFY_BRIDGE = "br-3f9a0c1d2e4b";

/** A packet for this host: raw prerouting, then input if prerouting let it through. */
function delivered(packet: Packet): Verdict {
  if (evaluateChain(chain("prerouting"), packet) === "drop") return "drop";
  return evaluateChain(chain("input"), packet);
}

type Flow = Pick<Packet, "protocol"> & Partial<Packet>;

const fromBridge = (saddr: string, flow: Flow): Packet =>
  ({ iifname: GATEWAY_BRIDGE, saddr, daddr: VPN_ADDRESS, daddrIsLocal: true, ctState: "new", ...flow });

const SSH: Flow = { protocol: "tcp", dport: 22 };
const ECHO: Flow = { protocol: "icmp", icmpType: "echo-request" };

describe("the hub reaches this host's VPN address (VP-13)", () => {
  test("VP-13 the hub opens SSH to 100.64.0.1 and keeps the connection", () => {
    expect(delivered(fromBridge(HUB, SSH))).toBe("accept");
    expect(delivered(fromBridge(HUB, { ...SSH, ctState: "established" }))).toBe("accept");
  });

  test("VP-13 the hub pings 100.64.0.1", () => {
    expect(delivered(fromBridge(HUB, ECHO))).toBe("accept");
  });

  test("VPS-001 SSH over the VPN interface is unchanged", () => {
    expect(delivered({ iifname: "tailscale0", saddr: "100.64.0.2", daddr: VPN_ADDRESS, daddrIsLocal: true, ctState: "new", ...SSH })).toBe("accept");
  });
});

describe("no other source reaches this host's VPN address (VP-13, DNS-005)", () => {
  const others: [string, Packet][] = [
    ["guacd on the same bridge", fromBridge(GUACD, {})],
    ["the bridge's next address", fromBridge("172.30.0.11", {})],
    ["the hub's notify address", fromBridge(HUB_NOTIFY, { iifname: NOTIFY_BRIDGE })],
    ["Apprise", fromBridge(APPRISE, { iifname: NOTIFY_BRIDGE })],
    ["a container on docker0", fromBridge("172.17.0.2", { iifname: "docker0" })],
    ["the hub's address forged on the public interface", fromBridge(HUB, { iifname: "eth0" })],
    ["the hub's address on an interface that is not a Docker bridge", fromBridge(HUB, { iifname: "veth1a2b3c" })],
  ];

  for (const [name, packet] of others) {
    test(`DNS-005 ${name} may not open SSH to or ping 100.64.0.1`, () => {
      expect(delivered({ ...packet, ...SSH })).toBe("drop");
      expect(delivered({ ...packet, ...ECHO })).toBe("drop");
    });
  }
});

describe("the hub reaches nothing else on this host (VP-13, DNS-005, VPS-002)", () => {
  test.each([
    ["tcp", 443],
    ["tcp", 9090],
    ["tcp", 8080],
    ["tcp", 5900],
    ["tcp", 2222],
    ["tcp", 23],
    ["udp", 22],
    ["udp", 53],
    ["udp", 3478],
    ["udp", 41641],
  ] as const)("DNS-005 the hub may not reach 100.64.0.1 on %s/%d", (protocol, dport) => {
    expect(delivered(fromBridge(HUB, { protocol, dport }))).toBe("drop");
    expect(delivered(fromBridge(HUB, { protocol, dport, ctState: "established" }))).toBe("drop");
  });

  test("DNS-005 the hub sends no other ICMP type to 100.64.0.1", () => {
    for (const icmpType of ["timestamp-request", "echo-reply", "address-mask-request"]) {
      expect(delivered(fromBridge(HUB, { protocol: "icmp", icmpType }))).toBe("drop");
    }
  });

  test("VPS-002 the hub may not open SSH to the public address or its bridge gateway", () => {
    expect(delivered(fromBridge(HUB, { ...SSH, daddr: PUBLIC_ADDRESS }))).toBe("drop");
    expect(delivered(fromBridge(HUB, { ...SSH, daddr: "172.30.0.1" }))).toBe("drop");
  });
});

describe("the hub addresses in the firewall and in compose.yaml (VP-13)", () => {
  type Attachment = { ipv4_address?: string; gw_priority?: number } | null;
  const compose = Bun.YAML.parse(readFileSync(join(import.meta.dir, "..", "..", "compose.yaml"), "utf8")) as {
    services: { hub: { networks: Record<string, Attachment> } };
  };
  const networks = compose.services.hub.networks;

  test("VP-13 hub_source is the hub's fixed gateway address, and gateway holds its default route", () => {
    const address = /^\$\{ANIMA_HUB_IP:-(\S+)\}$/.exec(networks["gateway"]?.ipv4_address ?? "")?.[1];
    expect(ruleset.commands).toContain(`define hub_source = ${String(address)}`);
    // Docker gives the default route to the highest gw_priority (default 0) among the networks
    // that have a gateway; gateway's is above every other network's.
    const priority = (name: string): number => networks[name]?.gw_priority ?? 0;
    const others = Object.keys(networks).filter((name) => name !== "gateway");
    expect(others.length).toBeGreaterThan(0);
    for (const name of others) expect(priority("gateway")).toBeGreaterThan(priority(name));
  });

  test("VP-13 hub_vpn_address is one address in the VPN range", () => {
    expect(ruleset.commands).toContain(`define hub_vpn_address = ${VPN_ADDRESS}`);
  });
});

describe("packet evaluator ICMP types", () => {
  const packet: Packet = { iifname: "br-x", saddr: HUB, daddr: VPN_ADDRESS, daddrIsLocal: true, ctState: "new", protocol: "icmp", icmpType: "echo-request" };
  const one = (rule: string): Chain => ({ table: "inet x", name: "c", rules: [rule], policy: "accept" });

  test("VP-13 an icmp type rule matches by name, in a set and negated", () => {
    expect(evaluateChain(one("icmp type echo-request drop"), packet)).toBe("drop");
    expect(evaluateChain(one("icmp type echo-reply drop"), packet)).toBe("accept");
    expect(evaluateChain(one("icmp type { time-exceeded, echo-request } drop"), packet)).toBe("drop");
    expect(evaluateChain(one("icmp type != echo-request drop"), packet)).toBe("accept");
    expect(evaluateChain(one("icmpv6 type echo-request drop"), packet)).toBe("accept");
    expect(evaluateChain(one("icmp type echo-request drop"), { ...packet, protocol: "tcp", dport: 22 })).toBe("accept");
  });

  test("VP-13 the evaluator refuses an ICMP packet with no type", () => {
    expect(() => evaluateChain(one("icmp type echo-request drop"), { ...packet, icmpType: undefined })).toThrow("no icmpType");
  });
});
