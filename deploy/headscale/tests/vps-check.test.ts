import { beforeEach, describe, expect, test } from "bun:test";
import { ALLOWED_SS, fakeHost, mutatingCalls, type Host } from "./vps-host.ts";

let host: Host;

beforeEach(() => {
  host = fakeHost();
  host.harden();
});

describe("harden.sh check", () => {
  test("VPS-002: check passes on a hardened host with only allowed listeners", () => {
    const run = host.run(["check"]);
    expect(run.out).toEndWith("== End of inventory\nok: no violations\n");
    expect(run.code).toBe(0);
  });

  test("VPS-005: check changes nothing, even with violations", () => {
    const unhardened = fakeHost();
    const before = unhardened.tree();
    const run = unhardened.run(["check"], { FAKE_FIREWALLD_ACTIVE: "0", FAKE_FIREWALLD: "enabled" });
    expect(run.code).toBe(1);
    expect(mutatingCalls(run)).toEqual([]);
    expect(unhardened.tree()).toEqual(before);
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

  test("VPS-002: a port in the local allow file is not a violation", () => {
    host.setListeners([...ALLOWED_SS, "tcp LISTEN 0 244 0.0.0.0:8080 0.0.0.0:*"]);
    host.write("etc/anima/firewall-allow", "# web site\n8080/tcp\n");
    expect(host.run(["check"]).code).toBe(0);
  });

  test.each([
    ["tailscale0 is missing", { FAKE_TAILSCALE: "1" }],
    ["table inet anima_filter is not loaded", { FAKE_NFT_TABLE: "1" }],
    ["nftables.service is not enabled", { FAKE_ENABLED: "1" }],
    ["firewalld is not masked", { FAKE_FIREWALLD: "enabled" }],
    ["SELinux is Permissive, not Enforcing", { FAKE_ENFORCE: "Permissive" }],
    ["/opt/anima/headscale/lib is labelled system_u:object_r:usr_t:s0, not container_file_t", { FAKE_LABEL: "system_u:object_r:usr_t:s0" }],
    ["Docker runs containers without SELinux labels", { FAKE_DOCKER_SECURITY: "[name=seccomp,profile=builtin]" }],
    ["dnf-automatic.timer is not enabled", { FAKE_ENABLED: "1" }],
  ])("VPS-002/VPS-004: check reports %s", (message, env) => {
    const run = host.run(["check"], env);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: ${message}`);
  });

  test("VPS-002: check reports a boot config that does not load the Anima rules", () => {
    host.write("etc/sysconfig/nftables.conf", '#include "/etc/nftables/anima.nft"\n');
    expect(host.run(["check"]).out).toContain("VIOLATION: /etc/sysconfig/nftables.conf does not load /etc/nftables/anima.nft at boot");
  });

  test.each([
    ["flush ruleset"],
    ['include "/etc/nftables/main.nft"'],
    ["  table inet other { }"],
  ])("VPS-002: check reports a boot config with another active line: %s", (line) => {
    host.write("etc/sysconfig/nftables.conf", `# stock\n${line}\ninclude "/etc/nftables/anima.nft"\n`);
    expect(host.run(["check"]).out).toContain(
      "VIOLATION: /etc/sysconfig/nftables.conf:2: only the Anima include may be active in this file",
    );
  });

  test("VPS-002: check reports a boot config that nft -c rejects", () => {
    const run = host.run(["check"], { FAKE_NFT_CHECK_BOOT: "1" });
    expect(run.calls).toContain(`NFT -c -f ${host.root}/etc/sysconfig/nftables.conf`);
    expect(run.out).toContain("VIOLATION: nft -c rejects /etc/sysconfig/nftables.conf; nftables.service would fail at boot");
  });

  test("VPS-002: check reports a missing nftables.service drop-in", () => {
    const unhardened = fakeHost();
    expect(unhardened.run(["check"]).out).toContain(
      "VIOLATION: nftables.service would flush the whole ruleset on stop or reload",
    );
  });

  test("VPS-002: check reports a local allow file with a line that is not one port", () => {
    host.write("etc/anima/firewall-allow", "80/tcp accept\n");
    expect(host.run(["check"]).out).toContain("VIOLATION: /etc/anima/firewall-allow has a line that is not one PORT/PROTO");
  });

  test.each([
    ["apply_updates = yes", "apply_updates = no"],
    ["upgrade_type = default", "upgrade_type = security"],
    ["reboot = when-needed", "reboot = never"],
  ])("VPS-004: check reports dnf-automatic without %p", (line, replacement) => {
    host.write("etc/dnf/automatic.conf", host.read("etc/dnf/automatic.conf").replace(line, replacement));
    expect(host.run(["check"]).out).toContain(`VIOLATION: /etc/dnf/automatic.conf: expected "${line}"`);
  });

  test.each([
    ["27.5.1", true],
    ["28.0.0", false],
  ])("D-10: check warns about Docker %p before 28: %p", (version, warns) => {
    const run = host.run(["check"], { FAKE_DOCKER_VERSION: version });
    expect(run.out.includes("warning: Docker")).toBe(warns);
    expect(run.code).toBe(0);
  });

  test.each([
    ['ID=ubuntu\nVERSION_ID="24.04"\n', true],
    ['ID="almalinux"\nVERSION_ID="9.6"\n', true],
    ['ID="almalinux"\nVERSION_ID="10.0"\n', false],
  ])("D-9: check warns on another OS (%p: %p) but does not fail for it", (release, warns) => {
    host.write("etc/os-release", release);
    const run = host.run(["check"]);
    expect(run.out.includes("warning: not AlmaLinux 10")).toBe(warns);
    expect(run.code).toBe(0);
  });
});

describe("harden.sh check inventory (VPS-005)", () => {
  const section = (out: string, title: string): string[] => {
    const lines = out.split("\n");
    const start = lines.findIndex((line) => line.startsWith(`== ${title}`));
    const end = lines.findIndex((line, i) => i > start && line.startsWith("=="));
    return lines.slice(start + 1, end);
  };

  test("marks a listener whose program is not expected, with its unit", () => {
    host.setListeners([
      'tcp LISTEN 0 128 0.0.0.0:22 0.0.0.0:* users:(("sshd",pid=900,fd=3))',
      'tcp LISTEN 0 511 127.0.0.1:3306 0.0.0.0:* users:(("mariadbd",pid=1200,fd=20))',
    ]);
    const lines = section(host.run(["check"], { FAKE_UNIT: "mariadb.service" }).out, "Listening sockets");
    expect(lines).toEqual(["  tcp 0.0.0.0:22 sshd mariadb.service", "! tcp 127.0.0.1:3306 mariadbd mariadb.service"]);
  });

  test("lists running containers", () => {
    const run = host.run(["check"], { FAKE_CONTAINERS: "blog wordpress:6 0.0.0.0:80->80/tcp" });
    expect(section(run.out, "Running containers")).toEqual(["blog wordpress:6 0.0.0.0:80->80/tcp"]);
  });

  test("names each bridge and marks one the forward chain would not let into the VPN", () => {
    const networks = "bridge bridge 2673f396533215b1e304\\napp bridge 6c7f1b24fa0e40e6770b\\nlegacy bridge 99aa";
    const run = host.run(["check"], { FAKE_NETWORKS: networks });
    expect(section(run.out, "Docker networks")).toEqual(["  bridge bridge docker0", "  app bridge br-6c7f1b24fa0e", "  legacy bridge br-99aa"]);
    const custom = host.run(["check"], { FAKE_NETWORKS: "web bridge 6c7f1b24fa0e", FAKE_BRIDGE: "webbr0" });
    expect(section(custom.out, "Docker networks")).toEqual(["! web bridge webbr0"]);
  });

  test("lists enabled services that are not stock", () => {
    const run = host.run(["check"], { FAKE_UNITS: "sshd.service enabled enabled\\nnginx.service enabled disabled" });
    expect(section(run.out, "Enabled services")).toEqual(["! nginx.service"]);
  });

  test("prints each active firewalld zone, or that firewalld is not running", () => {
    expect(section(host.run(["check"]).out, "firewalld")).toEqual(["firewalld: masked, not running"]);
    const active = host.run(["check"], { FAKE_FIREWALLD_ACTIVE: "0", FAKE_FIREWALLD: "enabled" });
    const zones = section(active.out, "firewalld");
    expect(zones.filter((line) => !line.startsWith(" "))).toEqual(["docker (active)", "public (active)"]);
    expect(zones).toContain("  services: ssh");
  });
});

describe("harden.sh updates", () => {
  test("VPS-004: installs dnf-automatic, its config and schedule, and enables the timer", () => {
    const fresh = fakeHost();
    const run = fresh.run(["updates"]);
    expect(run.code).toBe(0);
    expect(run.calls).toContain("DNF install -y dnf-automatic");
    expect(fresh.read("etc/dnf/automatic.conf")).toBe(host.read("etc/dnf/automatic.conf"));
    expect(fresh.exists("etc/systemd/system/dnf-automatic.timer.d/anima.conf")).toBe(true);
    const reload = run.calls.indexOf("SYSTEMCTL daemon-reload");
    expect(reload).toBeGreaterThanOrEqual(0);
    expect(run.calls.indexOf("SYSTEMCTL enable --now dnf-automatic.timer")).toBeGreaterThan(reload);
  });

  test("VPS-004: keeps the first copy of the original automatic.conf", () => {
    const fresh = fakeHost();
    fresh.write("etc/dnf/automatic.conf", "# stock\n");
    expect(fresh.run(["updates"]).code).toBe(0);
    expect(fresh.run(["updates"]).code).toBe(0);
    expect(fresh.read("var/lib/anima/automatic.conf.previous")).toBe("# stock\n");
  });
});
