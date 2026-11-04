import { beforeEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ALLOWED_SS, fakeHost, mutatingCalls, VPN_SESSION, VPS_DIR, type Host } from "./vps-host.ts";

let host: Host;

beforeEach(() => {
  host = fakeHost();
  host.harden();
});

describe("harden.sh check", () => {
  test("VPS-002: check passes on a hardened host with only allowed listeners", () => {
    const run = host.run(["check"]);
    expect(run.out).toBe("ok: no violations\n");
    expect(run.code).toBe(0);
  });

  test.each([
    ["tcp LISTEN 0 244 0.0.0.0:5432 0.0.0.0:*", true],
    ["tcp LISTEN 0 244 *:8080 *:*", true],
    ["udp UNCONN 0 0 [fe80::1%eth0]:546 [::]:*", true],
    ["tcp LISTEN 0 244 100.128.0.1:5432 0.0.0.0:*", true],
    ["tcp LISTEN 0 244 203.0.113.7:3478 0.0.0.0:*", true],
    ["tcp LISTEN 0 244 100.64.0.1:5432 0.0.0.0:*", false],
    ["tcp LISTEN 0 244 127.0.0.1:5432 0.0.0.0:*", false],
    ["tcp LISTEN 0 244 [::1]:5432 [::]:*", false],
    ["udp UNCONN 0 0 0.0.0.0%tailscale0:5353 0.0.0.0:*", false],
  ])("VPS-002: check treats listener %p as public: %p", (line, violation) => {
    host.setListeners([...ALLOWED_SS, line]);
    const run = host.run(["check"]);
    expect(run.code).toBe(violation ? 1 : 0);
    expect(run.out.match(/VIOLATION: public listener/g)?.length ?? 0).toBe(violation ? 1 : 0);
  });

  test.each([
    ["tailscale0 is missing", { FAKE_TAILSCALE: "1" }],
    ["table inet anima_filter is not loaded", { FAKE_NFT_TABLE: "1" }],
    ["nftables.service is not enabled", { FAKE_ENABLED: "1" }],
  ])("VPS-002: check reports %s", (message, env) => {
    const run = host.run(["check"], env);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: ${message}`);
  });

  test("VPS-002: check reports a missing nftables.service drop-in", () => {
    const unhardened = fakeHost();
    expect(unhardened.run(["check"]).out).toContain("VIOLATION: nftables.service would flush the whole ruleset on stop");
  });

  test.each([
    ["27.5.1", true],
    ["28.0.0", false],
  ])("D-10: check warns about Docker %p before 28: %p", (version, warns) => {
    const run = host.run(["check"], { FAKE_DOCKER_VERSION: version });
    expect(run.out.includes("warning: Docker")).toBe(warns);
    expect(run.code).toBe(0);
  });

  test("VPS-002: check warns on another OS but does not fail for it", () => {
    host.write("etc/os-release", 'ID=debian\nVERSION_ID="12"\n');
    const run = host.run(["check"]);
    expect(run.out).toContain("warning: not Ubuntu 24.04");
    expect(run.code).toBe(0);
  });
});

describe("harden.sh updates and docker", () => {
  test("VPS-004: updates installs the package, both files and runs a dry run", () => {
    const run = host.run(["updates"]);
    expect(run.code).toBe(0);
    expect(run.calls).toContain("APT_GET install -y unattended-upgrades");
    expect(host.read("etc/apt/apt.conf.d/52anima-unattended-upgrades")).toBe(
      readFileSync(join(VPS_DIR, "52anima-unattended-upgrades"), "utf8"),
    );
    expect(host.exists("etc/apt/apt.conf.d/20auto-upgrades")).toBe(true);
    expect(run.calls).toContain("UNATTENDED_UPGRADE --dry-run");
  });

  test("D-10: docker installs daemon.json with the loopback bind", () => {
    expect(host.run(["docker"]).code).toBe(0);
    expect(JSON.parse(host.read("etc/docker/daemon.json"))).toMatchObject({ ip: "127.0.0.1" });
  });

  test("D-10: merging keeps existing keys and forces the loopback bind", () => {
    host.write("etc/docker/daemon.json", '{"ip": "0.0.0.0", "data-root": "/srv/docker"}');
    expect(host.run(["docker"]).code).toBe(0);
    const merged = JSON.parse(host.read("etc/docker/daemon.json")) as Record<string, unknown>;
    expect(merged).toMatchObject({ ip: "127.0.0.1", "data-root": "/srv/docker" });
  });

  test.each([["{ broken"], ["[1]"], ['{"a": 1} {"b": 2}']])("D-10: refuses an existing daemon.json %p", (text) => {
    host.write("etc/docker/daemon.json", text);
    expect(host.run(["docker"]).code).toBe(2);
    expect(host.read("etc/docker/daemon.json")).toBe(text);
  });

  test("D-10: docker fails clearly without jq", () => {
    const run = host.run(["docker"], { HARDEN_JQ: "/nonexistent/jq" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("jq is required");
  });

  test("D-10: docker restarts Docker only when asked", () => {
    expect(host.run(["docker"]).calls).not.toContain("SYSTEMCTL restart docker");
    expect(host.run(["--restart-docker", "docker"]).calls).toContain("SYSTEMCTL restart docker");
  });
});

describe("harden.sh dry run, usage and lint", () => {
  test("VPS-005: --dry-run writes nothing and calls no mutating command", () => {
    host.write("var/lib/anima/nftables.pending", "pending\n");
    host.write("var/lib/anima/firewall-session", "100.64.0.2 50000 100.64.0.1 22\n");
    const before = host.tree();
    for (const command of ["ssh", "updates", "docker", "firewall", "confirm", "check"]) {
      const run = host.run(["--dry-run", "--restart-docker", command], { SSH_CONNECTION: VPN_SESSION });
      expect([command, run.code]).toEqual([command, 0]);
      expect([command, mutatingCalls(run)]).toEqual([command, []]);
    }
    expect(host.tree()).toEqual(before);
    expect(host.read("var/lib/anima/nftables.pending")).toBe("pending\n");
  });

  test.each([[[]], [["bogus"]], [["--bogus", "check"]], [["check", "ssh"]], [["--root"]]])(
    "usage errors exit 64: %p",
    (args) => {
      expect(host.run(args).code).toBe(64);
    },
  );

  test("--help prints the exit codes and exits 0", () => {
    const run = host.run(["--help"]);
    expect(run.code).toBe(0);
    expect(run.out).toContain("Exit codes: 0 done; 1");
  });

  test("CQ-024: harden.sh is shellcheck-clean", () => {
    const run = Bun.spawnSync(["shellcheck", join(VPS_DIR, "harden.sh")]);
    expect(run.stdout.toString()).toBe("");
    expect(run.exitCode).toBe(0);
  });
});
