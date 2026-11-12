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
const DATABASE_BRIDGE = "br-0db0db0db0db";

/** The interface of this host's route back to a source: the bridge whose subnet holds it. */
function routeTo(saddr: string): string {
  if (saddr.startsWith("172.30.0.")) return GATEWAY_BRIDGE;
  if (saddr.startsWith("172.30.3.")) return NOTIFY_BRIDGE;
  if (saddr.startsWith("172.17.")) return "docker0";
  return "eth0";
}

/** A packet for this host: raw prerouting, then input if prerouting let it through. */
function delivered(packet: Packet): Verdict {
  if (evaluateChain(chain("prerouting"), packet) === "drop") return "drop";
  return evaluateChain(chain("input"), packet);
}

type Flow = Pick<Packet, "protocol"> & Partial<Packet>;

const fromBridge = (saddr: string, flow: Flow): Packet =>
  ({ iifname: GATEWAY_BRIDGE, saddr, saddrRoute: routeTo(saddr), daddr: VPN_ADDRESS, daddrIsLocal: true, ctState: "new", ...flow });

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
    ["the hub's address forged on another Docker bridge", fromBridge(HUB, { iifname: DATABASE_BRIDGE })],
    ["the hub's address forged on docker0", fromBridge(HUB, { iifname: "docker0" })],
    ["the hub's address from a tailnet node", fromBridge(HUB, { iifname: "tailscale0" })],
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

  test.each([
    ["tcp", 443],
    ["udp", 41641],
    ["udp", 3478],
    ["tcp", 22],
  ] as const)("VP-13 the hub may not reach docker0's gateway or the public address on %s/%d", (protocol, dport) => {
    for (const daddr of ["172.17.0.1", "172.30.0.1", PUBLIC_ADDRESS]) {
      expect(delivered(fromBridge(HUB, { protocol, dport, daddr }))).toBe("drop");
    }
  });

  test("VP-13 replies to a connection this host opened to the hub still pass", () => {
    expect(delivered(fromBridge(HUB, { protocol: "tcp", dport: 51000, daddr: "172.30.0.1", ctState: "established" }))).toBe("accept");
  });
});

describe("the exception admits only well-formed hub packets (VP-13 security review)", () => {
  test("VP-13 F4 an invalid-state packet from the hub to 22 or as echo is dropped like every other invalid packet", () => {
    expect(delivered(fromBridge(HUB, { ...SSH, ctState: "invalid" }))).toBe("drop");
    expect(delivered(fromBridge(HUB, { ...ECHO, ctState: "invalid" }))).toBe("drop");
  });

  test("VP-13 F2 the hub's address is accepted only on the bridge its route goes back to", () => {
    // The real hub, but the route to 172.30.0.10 left the gateway bridge (a tailnet route over it):
    // the exception closes rather than trust the source.
    expect(delivered(fromBridge(HUB, { ...SSH, saddrRoute: "tailscale0" }))).toBe("drop");
    expect(delivered(fromBridge(HUB, { ...SSH, iifname: DATABASE_BRIDGE }))).toBe("drop");
    expect(delivered(fromBridge(HUB, { ...ECHO, iifname: DATABASE_BRIDGE }))).toBe("drop");
  });

  test("VP-13 F3 no tailnet node may send from an Anima container range, to this host or through it", () => {
    for (const saddr of [HUB, GUACD, "172.30.1.5", "172.30.255.254"]) {
      const packet = fromBridge(saddr, { ...SSH, iifname: "tailscale0", saddrRoute: "tailscale0" });
      expect(evaluateChain(chain("prerouting"), packet)).toBe("drop");
      expect(evaluateChain(chain("prerouting"), { ...packet, daddr: "172.30.0.10", daddrIsLocal: false })).toBe("drop");
    }
    const site = fromBridge("10.10.0.5", { ...SSH, iifname: "tailscale0", saddrRoute: "tailscale0" });
    expect(delivered(site)).toBe("accept");
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

describe("packet evaluator reverse path and short circuit", () => {
  const packet: Packet = { iifname: "br-x", saddr: HUB, daddr: VPN_ADDRESS, daddrIsLocal: true, protocol: "tcp", dport: 22 };
  const one = (rule: string): Chain => ({ table: "inet x", name: "c", rules: [rule], policy: "accept" });

  test("VP-13 fib saddr . iif oif exists holds only when the route back to the source is the input interface", () => {
    expect(evaluateChain(one("fib saddr . iif oif exists drop"), { ...packet, saddrRoute: "br-x" })).toBe("drop");
    expect(evaluateChain(one("fib saddr . iif oif exists drop"), { ...packet, saddrRoute: "br-y" })).toBe("accept");
    expect(() => evaluateChain(one("fib saddr . iif oif exists drop"), packet)).toThrow("no saddrRoute");
  });

  test("VP-13 a rule stops at the first match that fails, as in nft, but still refuses an unknown form", () => {
    expect(evaluateChain(one('iifname "eth0" fib saddr . iif oif exists drop'), packet)).toBe("accept");
    expect(() => evaluateChain(one('iifname "eth0" rt mtu 1 drop'), packet)).toThrow("unsupported match");
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
