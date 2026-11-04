import { beforeEach, describe, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fakeHost, RULESET, VPN_SESSION, type Host, type Run } from "./vps-host.ts";

const vpn = { SSH_CONNECTION: VPN_SESSION };
const second = { SSH_CONNECTION: "100.64.0.2 51001 100.64.0.1 22" };
const applied = (run: Run): boolean => run.calls.some((call) => /^NFT -f /.test(call));
const indexOf = (run: Run, re: RegExp): number => run.calls.findIndex((call) => re.test(call));
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh firewall", () => {
  test("VPS-002: firewall refuses without tailscale0", () => {
    const run = host.run(["firewall"], { ...vpn, FAKE_TAILSCALE: "1" });
    expect(run.code).toBe(2);
    expect(applied(run)).toBe(false);
  });

  test.each([
    ["100.63.255.1 51000 100.64.0.1 22", 2],
    ["100.128.0.1 51000 100.64.0.1 22", 2],
    ["fd7b:115c:a1e0::2 51000 fd7a:115c:a1e0::1 22", 2],
    ["203.0.113.9 51000 100.64.0.1 22", 2],
    ["100.64.0.2 51000 203.0.113.7 22", 2],
    ["fe80::1%eth0 51000 fe80::2%eth0 22", 2],
    ["", 2],
    ["::ffff:100.64.0.2 51000 100.64.0.1 22", 0],
    ["FD7A:115C:A1E0::2 51000 fd7a:115c:a1e0::1 22", 0],
  ])("VPS-002: firewall checks that SSH_CONNECTION %p is over the VPN (exit %p)", (connection, code) => {
    expect(host.run(["--dry-run", "firewall"], { SSH_CONNECTION: connection }).code).toBe(code);
  });

  test("VPS-002: --console skips the session check but not tailscale0", () => {
    expect(host.run(["--console", "--dry-run", "firewall"]).code).toBe(0);
    expect(host.run(["--console", "firewall"], { FAKE_TAILSCALE: "1" }).code).toBe(2);
  });

  test("VPS-002: firewall saves the ruleset, validates, arms a 180 s rollback, then applies", () => {
    const run = host.run(["firewall"], vpn);
    expect(run.code).toBe(0);
    expect(host.read("var/lib/anima/nftables.previous")).toBe(RULESET);
    const rollback = host.read("var/lib/anima/nftables.rollback");
    expect(rollback).toContain("delete table inet anima_filter\ntable inet anima_filter {");
    expect(rollback).not.toContain("table ip nat");
    const check = indexOf(run, /^NFT -c -f .*\/nftables\.conf$/);
    const arm = indexOf(
      run,
      /^SYSTEMD_RUN --unit=anima-firewall-rollback --on-active=180 .*\/bin\/sh -c touch '\/var\/lib\/anima\/firewall-rolled-back' && '[^']+' -f '\/var\/lib\/anima\/nftables\.rollback'$/,
    );
    const apply = indexOf(run, /^NFT -f /);
    expect(check).toBeGreaterThanOrEqual(0);
    expect(arm).toBeGreaterThan(check);
    expect(apply).toBeGreaterThan(arm);
    expect(host.exists("etc/nftables.conf")).toBe(false);
  });

  test.each([
    ["nft -c rejects nftables.conf", { FAKE_NFT_CHECK: "1" }],
    ["nft -c rejects the rollback", { FAKE_NFT_CHECK_ROLLBACK: "1" }],
  ])("VPS-002: firewall applies and arms nothing when %s", (_, env) => {
    const run = host.run(["firewall"], { ...vpn, ...env });
    expect(run.code).toBe(1);
    expect(applied(run)).toBe(false);
    expect(run.calls.some((call) => call.startsWith("SYSTEMD_RUN"))).toBe(false);
  });

  test("VPS-002: a failed apply stops the rollback timer", () => {
    const run = host.run(["firewall"], { ...vpn, FAKE_NFT_APPLY: "1" });
    expect(run.code).toBe(1);
    const stop = run.calls.findLastIndex((call) => call === "SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(stop).toBeGreaterThan(indexOf(run, /^NFT -f /));
  });

  test("VPS-002: a second run stops and resets the old rollback before arming", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["firewall"], vpn);
    const stop = indexOf(run, /^SYSTEMCTL stop anima-firewall-rollback\.timer$/);
    const reset = indexOf(run, /^SYSTEMCTL reset-failed anima-firewall-rollback\.service$/);
    expect(stop).toBeGreaterThanOrEqual(0);
    expect(reset).toBeGreaterThan(stop);
    expect(indexOf(run, /^SYSTEMD_RUN /)).toBeGreaterThan(reset);
  });
});

describe("harden.sh confirm", () => {
  beforeEach(() => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
  });

  test("VPS-002: confirm from a new VPN session stops the rollback and installs the rules for boot", () => {
    const original = readFileSync(join(host.vps, "nftables.conf"), "utf8");
    writeFileSync(join(host.vps, "nftables.conf"), "# edited after firewall\n");
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(0);
    expect(run.calls).toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.read("etc/nftables.conf")).toBe(original);
    expect(host.read("etc/systemd/system/nftables.service.d/anima.conf")).toBe(
      readFileSync(join(host.vps, "nftables-unit.conf"), "utf8"),
    );
    expect(indexOf(run, /^SYSTEMCTL daemon-reload$/)).toBeLessThan(indexOf(run, /^SYSTEMCTL enable nftables\.service$/));
    expect(host.exists("var/lib/anima/nftables.pending")).toBe(false);
  });

  test("VPS-002: confirm keeps the first copy of the original /etc/nftables.conf", () => {
    host.write("etc/nftables.conf", "# ubuntu default\n");
    expect(host.run(["confirm"], second).code).toBe(0);
    expect(host.run(["firewall"], vpn).code).toBe(0);
    expect(host.run(["confirm"], second).code).toBe(0);
    expect(host.read("var/lib/anima/nftables.conf.previous")).toBe("# ubuntu default\n");
  });

  test.each([
    ["the session that ran firewall", vpn],
    ["a session that is not over the VPN", { SSH_CONNECTION: "203.0.113.9 51001 203.0.113.7 22" }],
    ["a timer that is no longer waiting", { ...second, FAKE_SUBSTATE: "elapsed" }],
  ])("VPS-002: confirm refuses %s", (_, env) => {
    const run = host.run(["confirm"], env);
    expect(run.code).toBe(2);
    expect(host.exists("etc/nftables.conf")).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
  });

  test("VPS-002: confirm refuses once the rollback has started", () => {
    host.write("var/lib/anima/firewall-rolled-back", "");
    expect(host.run(["confirm"], second).code).toBe(2);
    expect(host.exists("etc/nftables.conf")).toBe(false);
  });

  test("VPS-002: --console confirms without an SSH session", () => {
    expect(host.run(["--console", "confirm"]).code).toBe(0);
  });
});
