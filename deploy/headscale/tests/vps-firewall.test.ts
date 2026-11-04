import { beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, ALLOWED_SS, fakeHost, RULESET, VPN_SESSION, type Host, type Run } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);

const vpn = { SSH_CONNECTION: VPN_SESSION };
const second = { SSH_CONNECTION: "100.64.0.2 51001 100.64.0.1 22" };
const applied = (run: Run): boolean => run.calls.some((call) => /^NFT -f /.test(call));
const armed = (run: Run): boolean => run.calls.some((call) => call.startsWith("SYSTEMD_RUN --unit="));
const indexOf = (run: Run, re: RegExp): number => run.calls.findIndex((call) => re.test(call));
const CONTEXT = "--property=SELinuxContext=system_u:system_r:unconfined_service_t:s0";
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

  test("VPS-002: firewall saves the ruleset, validates, proves and arms a 180 s rollback, then applies", () => {
    const run = host.run(["firewall"], vpn);
    expect(run.code).toBe(0);
    expect(host.read("var/lib/anima/nftables.previous")).toBe(RULESET);
    const rollback = host.read("var/lib/anima/nftables.rollback");
    expect(rollback).toContain("delete table inet anima_filter\ntable inet anima_filter {");
    expect(rollback).not.toContain("table ip nat");
    const check = indexOf(run, /^NFT -c -f \/dev\/stdin$/);
    const probe = indexOf(run, new RegExp(`^SYSTEMD_RUN --wait --pipe --quiet ${CONTEXT} /bin/sh -c '[^']+' -c -f '/var/lib/anima/nftables\\.rollback'$`));
    const arm = indexOf(
      run,
      new RegExp(`^SYSTEMD_RUN --unit=anima-firewall-rollback --on-active=180 .*${CONTEXT} /bin/sh -c touch '/var/lib/anima/firewall-rolled-back' && '[^']+' -f '/var/lib/anima/nftables\\.rollback'$`),
    );
    const apply = indexOf(run, /^NFT -f /);
    expect(check).toBeGreaterThanOrEqual(0);
    expect(probe).toBeGreaterThan(check);
    expect(arm).toBeGreaterThan(probe);
    expect(apply).toBeGreaterThan(arm);
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test.each([
    ["nft -c rejects the rules", { FAKE_NFT_CHECK: "1" }],
    ["nft -c rejects the rollback", { FAKE_NFT_CHECK_ROLLBACK: "1" }],
    ["a systemd unit cannot read the rollback (SELinux)", { FAKE_PROBE: "1" }],
  ])("VPS-002: firewall applies and arms nothing when %s", (_, env) => {
    const run = host.run(["firewall"], { ...vpn, ...env });
    expect(run.code).toBe(1);
    expect(applied(run)).toBe(false);
    expect(armed(run)).toBe(false);
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
    expect(indexOf(run, /^SYSTEMD_RUN --unit=/)).toBeGreaterThan(reset);
  });
});

describe("harden.sh firewall never silently closes an open port (VPS-005)", () => {
  const firewalld = { FAKE_FIREWALLD_ACTIVE: "0", FAKE_FIREWALLD: "enabled" };

  test.each([
    ["a public listener", { FAKE_SS_EXTRA: "tcp LISTEN 0 511 0.0.0.0:8080 0.0.0.0:*" }, "8080/tcp"],
    ["a firewalld port", { ...firewalld, FAKE_FW_PORTS: "8080/tcp" }, "8080/tcp"],
    ["a firewalld service", { ...firewalld, FAKE_FW_SERVICES: "ssh cockpit" }, "9090/tcp"],
    ["a firewalld port range", { ...firewalld, FAKE_FW_PORTS: "60000-61000/udp" }, "60000-61000/udp"],
    ["a firewalld rich rule", { ...firewalld, FAKE_FW_RICH: "rule port port=80 protocol=tcp accept" }, "public:rich-rules"],
    ["a firewalld source", { ...firewalld, FAKE_FW_SOURCES: "198.51.100.0/24" }, "public:sources"],
    ["a firewalld protocol", { ...firewalld, FAKE_FW_PROTOCOLS: "gre" }, "public:protocols"],
    ["a firewalld forward port", { ...firewalld, FAKE_FW_FORWARD_PORTS: "port=80:proto=tcp:toport=8080:toaddr=" }, "public:forward-ports"],
    ["a firewalld source port", { ...firewalld, FAKE_FW_SOURCE_PORTS: "53/udp" }, "public:source-ports"],
    ["firewalld masquerade", { ...firewalld, FAKE_FW_MASQ: "yes" }, "public:masquerade"],
    ["a zone target other than default", { ...firewalld, FAKE_FW_TARGET: "ACCEPT" }, "public:target"],
    ["a service that only includes others", { ...firewalld, FAKE_FW_SERVICES: "bundle" }, "service:bundle"],
    ["a firewalld policy of the operator's", { ...firewalld, FAKE_FW_POLICIES: "allow-host-ipv6\\nexpose\\n  ingress-zones: ANY" }, "policy:expose"],
    ["a firewalld direct rule", { ...firewalld, FAKE_FW_DIRECT: "ipv4 filter INPUT 0 -j ACCEPT\\n" }, "direct-rules"],
  ])("refuses for %s and changes nothing; --allow-closing lets it close", (_, env, port) => {
    const { FAKE_SS_EXTRA: extra, ...rest } = env as Record<string, string>;
    if (extra) host.setListeners([...ALLOWED_SS, extra]);
    const run = host.run(["firewall"], { ...vpn, ...rest });
    expect(run.code).toBe(2);
    expect(run.out).toContain(`would close: ${port} (`);
    expect(applied(run) || armed(run)).toBe(false);
    expect(host.exists("var/lib/anima/nftables.pending")).toBe(false);
    expect(host.run(["--allow-closing", `546/udp,${port}`, "firewall"], { ...vpn, ...rest }).code).toBe(0);
  });

  test.each([["docker-proxy"], ["dockerd"]])("a port %s publishes is noted, not refused: the table does not close it", (program) => {
    host.setListeners([...ALLOWED_SS, `tcp LISTEN 0 4096 0.0.0.0:8080 0.0.0.0:* users:(("${program}",pid=1300,fd=7))`]);
    const run = host.run(["firewall"], vpn);
    expect(run.code).toBe(0);
    expect(run.out).toContain("note: 8080/tcp (");
    expect(run.out).toContain("is published by Docker, not closed by this firewall; see guide 4.4");
    expect(run.out).not.toContain("would close");
  });

  test("SSH (22/tcp) and the Anima ports never count as closing", () => {
    host.setListeners([...ALLOWED_SS, "tcp LISTEN 0 128 0.0.0.0:22 0.0.0.0:*"]);
    expect(host.run(["firewall"], { ...vpn, ...firewalld, FAKE_FW_SERVICES: "ssh", FAKE_FW_PORTS: "443/tcp" }).code).toBe(0);
  });

  test.each([
    ["a forward port added to allow-host-ipv6", "allow-host-ipv6", { FAKE_FW_POLICY_EXTRA: "\\trule family=\"ipv4\" forward-port port=\"80\" protocol=\"tcp\" to-port=\"8080\"\\n" }],
    ["allow-host-ipv6 with target ACCEPT", "allow-host-ipv6", { FAKE_FW_POLICY_EDIT: "s/target: CONTINUE/target: ACCEPT/" }],
    ["allow-host-ipv6 at another priority", "allow-host-ipv6", { FAKE_FW_POLICY_EDIT: "s/priority: -15000/priority: 100/" }],
    ["a service added to docker-forwarding", "docker-forwarding", { FAKE_FW_POLICY_EDIT: "s/services: /services: ssh/" }],
    ["docker-forwarding from the public zone", "docker-forwarding", { FAKE_FW_POLICY_EDIT: "s/egress-zones: docker/egress-zones: public/" }],
  ])("refuses a stock policy changed in any way: %s", (_, policy, edit) => {
    const policies = `${policy}\\n  ingress-zones: ANY`;
    const run = host.run(["firewall"], { ...vpn, ...firewalld, FAKE_FW_POLICIES: policies, ...edit });
    expect(run.code).toBe(2);
    expect(run.out).toContain(`would close: policy:${policy} (`);
  });

  test("Docker's forwarding policy and the built-in allow-host-ipv6 are not openings", () => {
    const policies = "allow-host-ipv6\\n  ingress-zones: ANY\\ndocker-forwarding\\n  ingress-zones: ANY\\n  egress-zones: docker";
    expect(host.run(["firewall"], { ...vpn, ...firewalld, FAKE_FW_POLICIES: policies }).code).toBe(0);
  });

  test("Docker's own firewalld zone is not an opening", () => {
    const zones = "docker\\n  interfaces: docker0";
    expect(host.run(["firewall"], { ...vpn, ...firewalld, FAKE_ZONES: zones, FAKE_FW_PORTS: "8080/tcp" }).code).toBe(0);
  });

  test("a port in the local allow file stays open and is not closing", () => {
    host.setListeners([...ALLOWED_SS, "tcp LISTEN 0 511 0.0.0.0:8080 0.0.0.0:*"]);
    host.write("etc/anima/firewall-allow", "# blog\n8080/tcp\n");
    const run = host.run(["firewall"], vpn);
    expect(run.code).toBe(0);
    expect(host.checkedRules()).toContain("\t\ttcp dport 8080 accept\n");
    expect(host.read("var/lib/anima/nftables.pending")).toBe(host.checkedRules());
  });

  test("confirm checks again, so masking firewalld cannot close a port either", () => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
    const run = host.run(["confirm"], { ...second, ...firewalld, FAKE_FW_SERVICES: "cockpit" });
    expect(run.code).toBe(2);
    expect(run.calls).not.toContain("SYSTEMCTL mask firewalld.service");
  });
});

describe("harden.sh confirm", () => {
  beforeEach(() => {
    expect(host.run(["firewall"], vpn).code).toBe(0);
  });

  test("VPS-002: confirm from a new VPN session stops the rollback, installs the rules for boot and masks firewalld", () => {
    const pending = host.read("var/lib/anima/nftables.pending");
    writeFileSync(join(host.vps, "nftables.conf"), "# edited after firewall\n");
    host.write("etc/sysconfig/nftables.conf", '#include "/etc/nftables/main.nft"\n');
    const run = host.run(["confirm"], second);
    expect(run.code).toBe(0);
    expect(run.calls).toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
    expect(host.read("etc/nftables/anima.nft")).toBe(pending);
    expect(host.read("etc/sysconfig/nftables.conf")).toStartWith('#include "/etc/nftables/main.nft"\n');
    expect(host.read("etc/sysconfig/nftables.conf")).toEndWith('\ninclude "/etc/nftables/anima.nft"\n');
    expect(host.read("etc/systemd/system/nftables.service.d/anima.conf")).toBe(
      readFileSync(join(host.vps, "nftables-unit.conf"), "utf8"),
    );
    expect(indexOf(run, /^SYSTEMCTL daemon-reload$/)).toBeLessThan(indexOf(run, /^SYSTEMCTL enable nftables\.service$/));
    const mask = indexOf(run, /^SYSTEMCTL mask firewalld\.service$/);
    expect(mask).toBeGreaterThan(indexOf(run, /^SYSTEMCTL enable nftables\.service$/));
    expect(indexOf(run, /^SYSTEMCTL stop firewalld\.service$/)).toBeGreaterThan(mask);
    expect(host.exists("var/lib/anima/nftables.pending")).toBe(false);
  });

  test("VPS-002: confirm adds the boot include once and keeps the first copy of the original", () => {
    host.write("etc/sysconfig/nftables.conf", "# stock\n");
    expect(host.run(["confirm"], second).code).toBe(0);
    expect(host.run(["firewall"], vpn).code).toBe(0);
    expect(host.run(["confirm"], second).code).toBe(0);
    expect(host.read("etc/sysconfig/nftables.conf").match(/^include /gm)).toHaveLength(1);
    expect(host.read("var/lib/anima/nftables.conf.previous")).toBe("# stock\n");
  });

  test("VPS-002: confirm records that firewalld was running, for the recovery", () => {
    expect(host.run(["confirm"], { ...second, FAKE_FIREWALLD_ACTIVE: "0" }).code).toBe(0);
    expect(host.exists("var/lib/anima/firewalld-was-active")).toBe(true);
  });

  test.each([
    ["the session that ran firewall", vpn],
    ["a session that is not over the VPN", { SSH_CONNECTION: "203.0.113.9 51001 203.0.113.7 22" }],
    ["a timer that is no longer waiting", { ...second, FAKE_SUBSTATE: "elapsed" }],
  ])("VPS-002: confirm refuses %s", (_, env) => {
    const run = host.run(["confirm"], env);
    expect(run.code).toBe(2);
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL stop anima-firewall-rollback.timer");
  });

  test("VPS-002: confirm refuses once the rollback has started", () => {
    host.write("var/lib/anima/firewall-rolled-back", "");
    expect(host.run(["confirm"], second).code).toBe(2);
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("VPS-002: confirm fails when the rollback starts while the timer is being stopped", () => {
    const run = host.run(["confirm"], { ...second, FAKE_STOP_STARTS_ROLLBACK: "1" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("the rollback started while confirming");
    expect(host.exists("etc/nftables/anima.nft")).toBe(false);
  });

  test("VPS-002: --console confirms without an SSH session and warns that SSH was not proven", () => {
    const run = host.run(["--console", "confirm"]);
    expect(run.code).toBe(0);
    expect(run.out).toContain("WARNING: --console skips the check that SSH over the VPN works");
    expect(run.out).toContain("test a NEW SSH login over the VPN");
  });
});
