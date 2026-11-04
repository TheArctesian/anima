import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { interfacePorts, NftParseError, parseNftables, publicPorts, unexplainedAccepts, type Chain } from "../src/nftables.ts";
import { VPS_DIR } from "./vps-host.ts";

const readVps = (name: string): string => readFileSync(join(VPS_DIR, name), "utf8");
const DSTNAT_PRIORITY = -100;

describe("nftables.conf", () => {
  const ruleset = parseNftables(readVps("nftables.conf"));
  const hooked = (hook: string): Chain => ruleset.chains.find((chain) => chain.hook === hook) as Chain;
  const input = hooked("input");

  test("VPS-002: the input chain drops by default", () => {
    expect(input.table).toBe("inet anima_filter");
    expect(input.policy).toBe("drop");
  });

  test("VPS-002: public accepts are exactly 443/tcp, 3478/udp and 41641/udp", () => {
    expect(publicPorts(input).sort()).toEqual(["3478/udp", "41641/udp", "443/tcp"]);
    expect(unexplainedAccepts(input)).toEqual([]);
  });

  test("VPS-002: 22/tcp is accepted only on tailscale0", () => {
    expect(interfacePorts(input, "tailscale0")).toEqual(["22/tcp"]);
    expect(publicPorts(input)).not.toContain("22/tcp");
  });

  test("VPS-002: replaces only its own table and never flushes the ruleset", () => {
    expect(ruleset.commands).toEqual(["table inet anima_filter", "delete table inet anima_filter"]);
    expect(readVps("nftables.conf")).not.toMatch(/^\s*flush\s+ruleset/m);
  });

  test("VPS-002: no forward chain with a drop policy (Docker owns forwarding)", () => {
    expect(ruleset.chains.filter((chain) => chain.hook === "forward" && chain.policy === "drop")).toEqual([]);
  });

  const VPN_DROPS = [
    'iifname != { "lo", "tailscale0" } ip daddr 100.64.0.0/10 drop',
    'iifname != { "lo", "tailscale0" } ip6 daddr fd7a:115c:a1e0::/48 drop',
  ];

  test("DNS-005: prerouting drops VPN and loopback destinations before Docker's DNAT", () => {
    const prerouting = hooked("prerouting");
    expect(prerouting.table).toBe("inet anima_filter");
    expect(prerouting.priority).toBeLessThanOrEqual(DSTNAT_PRIORITY);
    expect(prerouting.rules).toEqual(expect.arrayContaining([...VPN_DROPS, 'iifname != "lo" ip daddr 127.0.0.0/8 drop']));
  });

  test("VPS-002/DNS-005: the input chain drops VPN destinations off tailscale0 before any accept", () => {
    const firstAccept = input.rules.findIndex((rule) => rule.endsWith("accept"));
    for (const rule of VPN_DROPS) {
      expect(input.rules.indexOf(rule)).toBeGreaterThanOrEqual(0);
      expect(input.rules.indexOf(rule)).toBeLessThan(firstAccept);
    }
  });

  test.each([
    ["an unclosed table", "table inet t {\n chain input {\n"],
    ["a multi-line set", "table inet t {\n chain input {\n tcp dport {\n 22 } accept\n }\n}\n"],
    ["a stray brace", "}\n"],
    ["an unknown priority", "table inet t {\n chain c {\n type filter hook input priority loud;\n }\n}\n"],
  ])("VPS-002: the parser refuses %s instead of guessing", (_, text) => {
    expect(() => parseNftables(text)).toThrow(NftParseError);
  });

  test.each([
    ["raw", -300],
    ["dstnat - 5", -105],
    ["filter + 10", 10],
    ["-150", -150],
  ])("VPS-002: the parser reads priority %p as %p", (priority, value) => {
    const text = `table inet t {\n chain c {\n type filter hook input priority ${priority}; policy accept;\n }\n}\n`;
    expect(parseNftables(text).chains[0]?.priority).toBe(value);
  });
});

describe("configuration files", () => {
  test("VPS-002: the nftables.service drop-in stops by deleting only the Anima table", () => {
    const dropIn = readVps("nftables-unit.conf");
    const settings = dropIn.split("\n").filter((line) => !line.startsWith("#"));
    expect(settings.filter((line) => line.includes("flush"))).toEqual([]);
    expect(dropIn).toMatch(/^\[Service\]\nExecStop=\nExecStop=-\/usr\/sbin\/nft delete table inet anima_filter$/m);
  });

  test("VPS-001: the sshd drop-in disables password, keyboard-interactive and root login", () => {
    const lines = readVps("sshd-anima.conf").split("\n");
    for (const line of ["PasswordAuthentication no", "KbdInteractiveAuthentication no", "PermitRootLogin no"]) {
      expect(lines).toContain(line);
    }
    expect(lines).toContain("AuthenticationMethods publickey");
  });

  test("VPS-004: unattended upgrades run daily with security origins and reboot at 04:30", () => {
    const periodic = readVps("20auto-upgrades");
    expect(periodic).toContain('APT::Periodic::Update-Package-Lists "1";');
    expect(periodic).toContain('APT::Periodic::Unattended-Upgrade "1";');
    const upgrades = readVps("52anima-unattended-upgrades");
    expect(upgrades).toContain('"${distro_id}:${distro_codename}-security";');
    expect(upgrades).toContain('"${distro_id}ESM:${distro_codename}-infra-security";');
    expect(upgrades).toContain('Unattended-Upgrade::Automatic-Reboot "true";');
    expect(upgrades).toContain('Unattended-Upgrade::Automatic-Reboot-Time "04:30";');
    expect(upgrades).toContain('Unattended-Upgrade::Remove-Unused-Dependencies "true";');
  });

  test("VPS-004: Docker and Tailscale repositories are upgraded too", () => {
    const upgrades = readVps("52anima-unattended-upgrades");
    expect(upgrades).toMatch(/Origins-Pattern \{[^}]*"origin=Docker";[^}]*"origin=Tailscale";/);
  });

  test("D-10: daemon.json binds published ports to 127.0.0.1 by default", () => {
    expect(JSON.parse(readVps("daemon.json"))).toMatchObject({ ip: "127.0.0.1", "live-restore": true });
  });
});
