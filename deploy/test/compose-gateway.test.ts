// The fixed addresses on deploy/compose.yaml's gateway network (VP-13): the VPS firewall and sshd
// admit the hub's 172.30.0.10, so no other container may ever hold it. Nothing here starts Docker.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

type Attachments = Record<string, { ipv4_address?: string } | null>;
interface Compose {
  services: Record<string, { networks?: string[] | Attachments }>;
  networks: Record<string, { ipam?: { config?: { subnet: string }[] } } | null>;
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
