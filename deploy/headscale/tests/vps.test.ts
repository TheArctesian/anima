import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { interfacePorts, NftParseError, parseNftables, publicPorts, unexplainedAccepts, type Chain } from "../src/nftables.ts";
import { VPS_DIR } from "./vps-host.ts";

const readVps = (name: string): string => readFileSync(join(VPS_DIR, name), "utf8");
const DSTNAT_PRIORITY = -100;

const ruleset = parseNftables(readVps("nftables.conf"));
const hooked = (hook: string): Chain => ruleset.chains.find((chain) => chain.hook === hook) as Chain;
const input = hooked("input");

// VP-13: the hub container's SSH and ping to this host's VPN address, the one exception to DNS-005,
// each only when the route back to the hub is the bridge the packet came in on.
const HUB = 'iifname "br-*" ip saddr $hub_source ip daddr $hub_vpn_address';
const REVERSE_PATH = "fib saddr . iif oif exists";
const HUB_SSH = `${HUB} tcp dport 22 ${REVERSE_PATH} accept`;
const HUB_ECHO_PREROUTING = `${HUB} icmp type echo-request ${REVERSE_PATH} accept`;
const HUB_ECHO_INPUT = `${HUB} icmp type echo-request ${REVERSE_PATH} limit rate 5/second burst 10 packets accept`;
const HUB_OTHERWISE = 'iifname "br-*" ip saddr $hub_source drop';

describe("nftables.conf", () => {
  test("VPS-002: the input chain drops by default", () => {
    expect(input.table).toBe("inet anima_filter");
    expect(input.policy).toBe("drop");
  });

  test("VPS-002: public accepts are exactly 443/tcp, 3478/udp and 41641/udp", () => {
    expect(publicPorts(input).sort()).toEqual(["3478/udp", "41641/udp", "443/tcp"]);
    // VP-13: the hub's ping is the one ICMP accept tied to an interface (vps-hub.test.ts).
    expect(unexplainedAccepts(input)).toEqual([HUB_ECHO_INPUT]);
  });

  test("VPS-002: the shell's ALLOWED_PUBLIC is the table's public ports plus SSH on the VPN", () => {
    const shell = /^ALLOWED_PUBLIC="([^"]*)"$/m.exec(readVps("open-ports.sh"))?.[1]?.trim().split(/\s+/) ?? [];
    expect(shell.sort()).toEqual([...publicPorts(input), ...interfacePorts(input, "tailscale0")].sort());
  });

  test("VPS-002 VP-13: 22/tcp is accepted only on tailscale0 and, for the hub container, on a Docker bridge", () => {
    expect(interfacePorts(input, "tailscale0")).toEqual(["22/tcp"]);
    expect(interfacePorts(input, "br-*")).toEqual(["22/tcp"]);
    expect(input.rules.filter((rule) => rule.startsWith('iifname "br-*"'))).toEqual([HUB_SSH, HUB_ECHO_INPUT, HUB_OTHERWISE]);
    expect(publicPorts(input)).not.toContain("22/tcp");
  });

  test("VPS-002: replaces only its own table and never flushes the ruleset", () => {
    expect(ruleset.commands.slice(0, 2)).toEqual(["table inet anima_filter", "delete table inet anima_filter"]);
    // NOT-002, VP-13: the only other top-level lines are the Apprise, hub and anima-probe defines
    // (vps-apprise.test.ts, vps-hub.test.ts, vps-probe-egress.test.ts).
    expect(ruleset.commands.slice(2).filter((command) => !/^define \w+ = /.test(command))).toEqual([]);
    expect(readVps("nftables.conf")).not.toMatch(/^\s*flush\s+ruleset/m);
  });

  test("VPS-002: no forward chain with a drop policy (Docker owns forwarding)", () => {
    expect(ruleset.chains.filter((chain) => chain.hook === "forward" && chain.policy === "drop")).toEqual([]);
  });

  // These tests read the rules as text; they cannot prove what packets do. The packet
  // behaviour (neighbour blocked, VPN peer reaches a DNAT-published port, container
  // reaches a tailnet peer) was checked in network namespaces on ubuntu:24.04 (T3) and
  // again on almalinux:10 with nftables 1.1.5 and Docker 29.9 (T7). The Apprise rules were
  // checked the same way on alpine:3.22 with nftables 1.1.3, br_netfilter off and on (WP-52 T5).
});

describe("nftables.conf keeps the VPN addresses on tailscale0 (DNS-005)", () => {
  const OFF_VPN = 'iifname != { "lo", "tailscale0" }';
  const LOCAL_VPN_DROPS = [
    `${OFF_VPN} fib daddr type local ip daddr 100.64.0.0/10 drop`,
    `${OFF_VPN} fib daddr type local ip6 daddr fd7a:115c:a1e0::/48 drop`,
  ];

  test("DNS-005: prerouting drops the host's own VPN and loopback addresses before Docker's DNAT", () => {
    const prerouting = hooked("prerouting");
    expect(prerouting.table).toBe("inet anima_filter");
    expect(prerouting.priority).toBeLessThanOrEqual(DSTNAT_PRIORITY);
    expect(prerouting.rules).toEqual([
      'iifname "tailscale0" ip saddr $hub_networks drop',
      HUB_SSH,
      HUB_ECHO_PREROUTING,
      ...LOCAL_VPN_DROPS,
      'iifname != "lo" ip daddr 127.0.0.0/8 drop',
      'iifname != { "lo", "tailscale0", "docker0", "br-*" } ip daddr 172.16.0.0/12 drop',
    ]);
  });

  test("NET-002: Docker's private range is reachable only from lo, the VPN and the Docker bridges, as the forward chain allows", () => {
    const set = (rule = ""): string | undefined => /iifname != (\{[^}]*\})/.exec(rule)?.[1];
    const docker = hooked("prerouting").rules.find((rule) => rule.includes("172.16.0.0/12"));
    expect(set(docker)).toBe('{ "lo", "tailscale0", "docker0", "br-*" }');
    expect(set(docker)).toBe(set(hooked("forward").rules[0]));
  });

  test("DNS-005: forwarding into tailscale0 is allowed only from the VPN, loopback and Docker bridges", () => {
    const forward = hooked("forward");
    expect(forward.table).toBe("inet anima_filter");
    expect(forward.policy).toBe("accept");
    expect(forward.rules[0]).toBe('oifname "tailscale0" iifname != { "lo", "tailscale0", "docker0", "br-*" } drop');
    // NOT-002: every later rule only narrows what Apprise may send (vps-apprise.test.ts).
    expect(forward.rules.slice(1).filter((rule) => !rule.startsWith("ip saddr $apprise "))).toEqual([]);
  });

  test("VPS-002/DNS-005: the input chain drops VPN destinations off tailscale0 before any accept but the hub's (VP-13)", () => {
    // VP-13 F4: invalid packets are dropped before the hub's exception can accept one.
    expect(input.rules.slice(0, 4)).toEqual(["ip saddr $apprise drop", "ct state invalid drop", HUB_SSH, HUB_ECHO_INPUT]);
    const firstAccept = input.rules.findIndex((rule, index) => index > 3 && rule.endsWith("accept"));
    for (const rule of [`${OFF_VPN} ip daddr 100.64.0.0/10 drop`, `${OFF_VPN} ip6 daddr fd7a:115c:a1e0::/48 drop`]) {
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
  const settings = (name: string): string[] =>
    readVps(name)
      .split("\n")
      .filter((line) => line !== "" && !line.startsWith("#"));

  test("VPS-002: the nftables.service drop-in clears EL10's flush on stop and on reload", () => {
    expect(settings("nftables-unit.conf")).toEqual([
      "[Service]",
      "ExecReload=",
      "ExecReload=/usr/sbin/nft -f /etc/sysconfig/nftables.conf",
      "ExecStop=",
    ]);
  });

  test("VPS-002: stopping or reloading nftables.service never deletes or flushes the live rules", () => {
    const actions = settings("nftables-unit.conf").filter((line) => /^Exec(Stop|Reload)=/.test(line));
    expect(actions.filter((line) => /delete|flush/.test(line))).toEqual([]);
  });

  test("VPS-001: the sshd drop-in disables password, keyboard-interactive and root login", () => {
    const lines = readVps("sshd-anima.conf").split("\n");
    for (const line of ["PasswordAuthentication no", "KbdInteractiveAuthentication no", "PermitRootLogin no"]) {
      expect(lines).toContain(line);
    }
    expect(lines).toContain("AuthenticationMethods publickey");
  });

  const automatic = (key: string): string | undefined =>
    settings("dnf-automatic.conf")
      .map((line) => /^(\w+) = (.*)$/.exec(line))
      .find((match) => match?.[1] === key)?.[2];

  test("VPS-004: dnf-automatic installs all updates, so Tailscale's (no advisories) install too", () => {
    expect(automatic("upgrade_type")).toBe("default");
    expect(automatic("download_updates")).toBe("yes");
    expect(automatic("apply_updates")).toBe("yes");
  });

  test("VPS-004: dnf-automatic reboots only when needed, at 04:30", () => {
    expect(automatic("reboot")).toBe("when-needed");
    expect(automatic("reboot_command")).toMatch(/^"shutdown -r 04:30 /);
  });

  test("VPS-004/VPS-006: updates run daily after the backup window, and before the 04:30 reboot", () => {
    const timer = settings("dnf-automatic-timer.conf");
    expect(timer.slice(0, 3)).toEqual(["[Timer]", "OnCalendar=", "OnCalendar=*-*-* 03:45:00 UTC"]);
    expect(timer).toContain("Persistent=true");
    const backup = readFileSync(join(VPS_DIR, "..", "backup", "headscale-backup.timer"), "utf8");
    expect(backup).toContain("OnCalendar=*-*-* 03:15:00 UTC");
    expect(backup).toContain("RandomizedDelaySec=15m");
    expect(timer).toContain("RandomizedDelaySec=10m");
  });

  test("D-10: daemon.json binds published ports to 127.0.0.1 and labels containers (D-9)", () => {
    expect(JSON.parse(readVps("daemon.json"))).toMatchObject({ ip: "127.0.0.1", "selinux-enabled": true, "live-restore": true });
  });

  test("NET-002: daemon.json forwards published ports by DNAT, so the hub sees the VPN client's address", () => {
    expect(JSON.parse(readVps("daemon.json"))).toMatchObject({ "userland-proxy": false });
  });
});
