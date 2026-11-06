import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, fakeHost, removeHosts, VPN_SESSION, type Host } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

// The guards around the boot file (S2, S3) and the sessions that may confirm (S4).
const vpn = { SSH_CONNECTION: VPN_SESSION };
const second = { SSH_CONNECTION: "100.64.0.2 51001 100.64.0.1 22" };
const ESTABLISHED = "0 0 100.64.0.1:22 100.64.0.2:51000\n0 0 [fd7a:115c:a1e0::1]:22 [FD7A:115C:A1E0::2]:52000\n";
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh firewall and confirm keep the boot file to the Anima include (VPS-002)", () => {
  test.each([["firewall"], ["confirm"]])("%s refuses while the boot file has another active line", (command) => {
    if (command === "confirm") expect(host.run(["firewall"], vpn).code).toBe(0);
    host.write("etc/sysconfig/nftables.conf", '# stock\ninclude "/etc/nftables/extra.nft"\n');
    const run = host.run([command], command === "confirm" ? second : vpn);
    expect(run.code).toBe(2);
    expect(run.out).toContain("/etc/sysconfig/nftables.conf:2: only the Anima include may be active");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("confirm checks the boot file with the pending rules before it changes anything", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    host.write("etc/sysconfig/nftables.conf", "# stock\n");
    const run = host.run(["confirm"], { ...second, FAKE_NFT_CHECK_BOOT: "1" });
    expect(run.code).toBe(2);
    expect(run.out).toContain("nft -c rejects the boot file with the new rules");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("confirm checks the installed boot file before it masks firewalld", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(0);
    const check = run.calls.indexOf(`NFT -c -f ${host.root}/etc/sysconfig/nftables.conf`);
    expect(check).toBeGreaterThan(run.calls.indexOf("SYSTEMCTL enable nftables.service"));
    expect(check).toBeLessThan(run.calls.indexOf("SYSTEMCTL mask firewalld.service"));
  });

  test("confirm masks firewalld that is not running or not installed, and stops nothing", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(0);
    expect(run.calls).toContain("SYSTEMCTL mask firewalld.service");
    expect(run.calls).not.toContain("SYSTEMCTL stop firewalld.service");
    expect(host.exists("var/lib/anima/firewalld-was-active")).toBe(false);
  });

  test("confirm stops a running firewalld and records that it ran", () => {
    const active = { FAKE_FIREWALLD_ACTIVE: "0", FAKE_FIREWALLD: "enabled" };
    expect(host.run(["firewall"], { ...vpn, ...active }).code).toBe(0);
    const run = host.run(["confirm"], { ...second, ...active });
    expect(run.code).toBe(0);
    expect(run.calls.indexOf("SYSTEMCTL stop firewalld.service")).toBeGreaterThan(run.calls.indexOf("SYSTEMCTL mask firewalld.service"));
    expect(host.exists("var/lib/anima/firewalld-was-active")).toBe(true);
  });
});

describe("harden.sh confirm needs a connection opened after firewall (VPS-002)", () => {
  test.each([
    ["the other session that was open over IPv4", "100.64.0.2 51000 100.64.0.1 22", "0 0 100.64.0.1:22 100.64.0.2:51000\n"],
    ["a session open over IPv6, in other case", "fd7a:115c:a1e0::2 52000 fd7a:115c:a1e0::1 22", ESTABLISHED],
    ["an IPv4-mapped session", "100.64.0.2 51001 100.64.0.1 22", "0 0 [::ffff:100.64.0.1]:22 [::ffff:100.64.0.2]:51001\n"],
  ])("refuses %s", (_, connection, established) => {
    expect(host.run(["firewall"], { SSH_CONNECTION: "100.64.0.2 49000 100.64.0.1 22", FAKE_SS_ESTABLISHED: established }).code).toBe(0);
    const run = host.run(["confirm"], { SSH_CONNECTION: connection });
    expect(run.code).toBe(2);
    expect(run.out).toContain("this SSH connection was open before firewall ran");
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
  });

  test("records the established SSH peers and accepts a later connection", () => {
    const run = host.run(["firewall"], { ...vpn, FAKE_SS_ESTABLISHED: ESTABLISHED });
    expect(run.calls).toContain("SS -Htn state established ( sport = :22 )");
    expect(host.read("var/lib/anima/firewall-peers")).toBe("100.64.0.2 51000\nfd7a:115c:a1e0::2 52000\n");
    expect(host.run(["confirm"], second).code).toBe(0);
  });

  test("refuses when firewall left no record of the connections", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    rmSync(join(host.root, "var/lib/anima/firewall-peers"));
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(2);
    expect(run.out).toContain("no recorded SSH connections");
  });
});

describe("harden.sh check and confirm read nftables.service as systemd loaded it (VPS-002, D-10)", () => {
  const OTHER = "/etc/systemd/system/nftables.service.d/zz-other.conf";
  const cases: [string, Record<string, string>, string][] = [
    [
      "a second drop-in that flushes on stop",
      { FAKE_NFT_UNIT_EDIT: `s#^DropInPaths=.*#& ${OTHER}#`, FAKE_NFT_UNIT_EXTRA: "ExecStop={ path=/sbin/nft ; argv[]=/sbin/nft flush ruleset ; ignore_errors=no }\\n" },
      `nftables.service has another drop-in: ${OTHER}`,
    ],
    [
      "a full override in /etc",
      { FAKE_NFT_UNIT_EDIT: "s#^FragmentPath=.*#FragmentPath=/etc/systemd/system/nftables.service#" },
      "nftables.service is not loaded from /usr/lib/systemd/system/nftables.service",
    ],
    ["an enabled iptables.service", { FAKE_IPTABLES: "enabled" }, "iptables.service is enabled"],
  ];

  test.each(cases)("check reports %s", (_, env, message) => {
    const hardened = fakeHost();
    hardened.harden();
    expect(hardened.run(["check"]).out).not.toContain("nftables.service");
    const run = hardened.run(["check"], env);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: ${message}`);
  });

  test.each(cases)("confirm refuses %s before it changes anything", (_, env, message) => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["confirm"], { ...second, ...env });
    expect(run.code).toBe(2);
    expect(run.out).toContain(message);
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
  });

  test("check reports the stock unit's flush on stop and reload", () => {
    const run = host.run(["check"]);
    expect(run.out).toContain('VIOLATION: nftables.service runs "ExecStop=/sbin/nft flush ruleset"');
    expect(run.out).toContain('VIOLATION: nftables.service runs "ExecReload=/sbin/nft flush ruleset; include');
  });

  test("confirm leaves firewalld running when the loaded unit still flushes after daemon-reload", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const extra = { FAKE_NFT_UNIT_EXTRA: "ExecStopPost={ path=/bin/sh ; argv[]=/bin/sh -c nft flush ruleset ; ignore_errors=no }\\n" };
    const run = host.run(["confirm"], { ...second, ...extra });
    expect(run.code).toBe(1);
    expect(run.out).toContain('nftables.service runs "ExecStopPost=/bin/sh -c nft flush ruleset"');
    expect(run.calls).not.toContain("SYSTEMCTL enable nftables.service");
    expect(run.calls).not.toContain("SYSTEMCTL mask firewalld.service");
  });
});
