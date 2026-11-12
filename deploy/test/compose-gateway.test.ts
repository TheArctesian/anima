// The fixed addresses on deploy/compose.yaml's gateway network (VP-13): the VPS firewall and sshd
// admit the hub's 172.30.0.10, so no other container may ever hold it. Every Anima network stays in
// 172.30.0.0/16, which the VPS firewall keeps off the VPN. Nothing here starts Docker.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

type Attachments = Record<string, { ipv4_address?: string } | null>;
interface Compose {
  services: Record<string, { networks?: string[] | Attachments; environment?: Record<string, string> }>;
  networks: Record<string, { ipam?: { config?: { subnet: string; ip_range?: string; gateway?: string }[] } } | null>;
}

const compose = Bun.YAML.parse(readFileSync(join(import.meta.dir, "..", "compose.yaml"), "utf8")) as Compose;

/** `${NAME:-default}` replaced by its default. */
const resolveDefault = (text: string): string => text.replace(/\$\{[A-Z_]+:-([^}]*)\}/g, "$1");

function ipv4(text: string): number {
  const parts = text.split(".").map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) throw new Error(`not an IPv4 address: ${text}`);
  return parts.reduce((sum, part) => sum * 256 + part, 0);
}

const gatewayAddress = (name: string): string | undefined => {
  const networks = compose.services[name]?.networks;
  return Array.isArray(networks) ? undefined : networks?.["gateway"]?.ipv4_address;
};

describe("fixed addresses on the gateway network (VP-13)", () => {
  test("VP-13 guacd has its own fixed gateway address, inside the subnet and not the hub's", () => {
    expect(gatewayAddress("guacd")).toBe("${ANIMA_GUACD_IP:-172.30.0.20}");
    const [base = "", bits = ""] = resolveDefault(compose.networks["gateway"]?.ipam?.config?.[0]?.subnet ?? "").split("/");
    const size = 2 ** (32 - Number(bits));
    const first = ipv4(base);
    const guacd = ipv4(resolveDefault(gatewayAddress("guacd") ?? ""));
    expect(guacd).not.toBe(ipv4(resolveDefault(gatewayAddress("hub") ?? "")));
    // Inside the subnet, and not its network address, the bridge gateway (.1) or the broadcast address.
    expect(guacd).toBeGreaterThan(first + 1);
    expect(guacd).toBeLessThan(first + size - 1);
  });

  test("VP-13 every service on the gateway network has a fixed address, so none can take the hub's", () => {
    const members = Object.entries(compose.services).filter(([, service]) =>
      Array.isArray(service.networks) ? service.networks.includes("gateway") : Object.keys(service.networks ?? {}).includes("gateway"));
    expect(members.map(([name]) => name).sort()).toEqual(["guacd", "hub"]);
    for (const [name] of members) expect(gatewayAddress(name)).toMatch(/^\$\{ANIMA_[A-Z]+_IP:-172\.30\.0\.\d+\}$/);
  });
});

/** The first address and size of a CIDR. */
function block(cidr: string): { first: number; size: number } {
  const [base = "", bits = ""] = cidr.split("/");
  return { first: ipv4(base), size: 2 ** (32 - Number(bits)) };
}

interface Ipam {
  subnet: string;
  ip_range?: string;
  gateway?: string;
}
const config = (network: string): Ipam | undefined => compose.networks[network]?.ipam?.config?.[0];

describe("dynamic addresses and Anima's network range (VP-13 security review)", () => {
  test("VP-13 I3 Docker hands out gateway addresses only above the hub's and guacd's fixed ones", () => {
    expect(config("gateway")?.ip_range).toBe("${ANIMA_HUB_IP_RANGE:-172.30.0.128/25}");
    // With ip_range Docker would take the range's first address as the gateway; it stays .1.
    expect(config("gateway")?.gateway).toBe("${ANIMA_HUB_GATEWAY:-172.30.0.1}");
    const subnet = block(resolveDefault(config("gateway")?.subnet ?? ""));
    const range = block(resolveDefault(config("gateway")?.ip_range ?? ""));
    expect(range.first).toBeGreaterThanOrEqual(subnet.first);
    expect(range.first + range.size).toBeLessThanOrEqual(subnet.first + subnet.size);
    for (const name of ["hub", "guacd"]) {
      const fixed = ipv4(resolveDefault(gatewayAddress(name) ?? ""));
      expect(fixed < range.first || fixed >= range.first + range.size).toBe(true);
    }
  });

  test("VP-13 every network has a fixed subnet inside 172.30.0.0/16, and no two overlap", () => {
    const anima = block("172.30.0.0/16");
    const blocks = Object.keys(compose.networks).map((name) => block(resolveDefault(config(name)?.subnet ?? `${name}/0`)));
    for (const { first, size } of blocks) {
      expect(first).toBeGreaterThanOrEqual(anima.first);
      expect(first + size).toBeLessThanOrEqual(anima.first + anima.size);
    }
    const sorted = blocks.sort((a, b) => a.first - b.first);
    for (let i = 1; i < sorted.length; i += 1) {
      const previous = sorted[i - 1] ?? { first: 0, size: 0 };
      expect(sorted[i]?.first ?? 0).toBeGreaterThanOrEqual(previous.first + previous.size);
    }
  });

  test("VP-13 I1 the hub's deny list holds lego's acme network and docker0", () => {
    const deny = (compose.services["hub"]?.environment?.["ANIMA_PROBE_DENY"] ?? "").split(",");
    expect(config("acme")?.subnet).toBe("${ANIMA_ACME_SUBNET:-172.30.4.0/24}");
    expect(deny).toContain("${ANIMA_ACME_SUBNET:-172.30.4.0/24}");
    expect(deny).toContain("172.17.0.0/16");
  });
});
