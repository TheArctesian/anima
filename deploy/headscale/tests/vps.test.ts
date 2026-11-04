import { beforeEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { interfacePorts, NftParseError, parseNftables, publicPorts, unexplainedAccepts, type Chain } from "../src/nftables.ts";

const vps = join(import.meta.dir, "..", "vps");
const script = join(vps, "harden.sh");
const readVps = (name: string): string => readFileSync(join(vps, name), "utf8");

const SSHD_T = [
  "passwordauthentication ${FAKE_SSHD_PASSWORD:-no}",
  "kbdinteractiveauthentication no",
  "permitrootlogin no",
  "permitemptypasswords no",
  "pubkeyauthentication yes",
  "authenticationmethods publickey",
  "x11forwarding no",
  "maxauthtries 3",
  "logingracetime 30",
  "allowusers $ANIMA_ADMIN_USER",
].join("\\n");

// Fake system commands (D-14): each logs its arguments, then acts as described.
const FAKES: Record<string, string> = {
  NFT: `case "$1 $2" in "list ruleset") cat "$FAKE_RULESET";; "-c -f") exit "\${FAKE_NFT_CHECK:-0}";; esac`,
  SYSTEMCTL: `[ "$1" = is-active ] && exit "\${FAKE_ACTIVE:-0}"; exit 0`,
  SYSTEMD_RUN: "exit 0",
  SSHD: `[ "$1" = -T ] && printf "${SSHD_T}\\n"; exit 0`,
  SS: `cat "$FAKE_SS"`,
  APT_GET: "exit 0",
  UNATTENDED_UPGRADE: "exit 0",
  IP: `exit "\${FAKE_TAILSCALE:-0}"`,
  ID: `echo "\${FAKE_GROUPS:-admin sudo}"`,
  GETENT: `[ "$2" = ghost ] && exit 2; [ "$2" = root ] && echo "root:x:0:0::/root:/bin/sh" && exit 0
echo "$2:x:1000:1000::/home/$2:/bin/bash"`,
};

const ALLOWED_SS = [
  'tcp LISTEN 0 4096 203.0.113.7:443 0.0.0.0:* users:(("headscale",pid=10,fd=3))',
  "udp UNCONN 0 0 203.0.113.7:3478 0.0.0.0:*",
  "udp UNCONN 0 0 0.0.0.0:41641 0.0.0.0:*",
  "udp UNCONN 0 0 [::]:41641 [::]:*",
  "tcp LISTEN 0 128 [::]:22 [::]:*",
  "tcp LISTEN 0 4096 127.0.0.1:9090 0.0.0.0:*",
  "tcp LISTEN 0 4096 127.0.0.53%lo:53 0.0.0.0:*",
  "tcp LISTEN 0 4096 100.64.0.1:443 0.0.0.0:*",
  "tcp LISTEN 0 4096 [fd7a:115c:a1e0::1]:443 [::]:*",
  "tcp LISTEN 0 4096 [::1]:50443 [::]:*",
];
const RULESET = "table ip nat {\n}\ntable inet anima_filter {\n\tchain input {\n\t}\n}\n";
const TEST_KEY = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestOnlyNotARealKey admin@test\n";

let work = "";
let root = "";

interface Run {
  code: number;
  out: string;
  calls: string[];
}

function harden(args: string[], env: Record<string, string> = {}): Run {
  const fakes = Object.fromEntries(Object.keys(FAKES).map((name) => [name, join(work, "bin", name)]));
  const result = Bun.spawnSync(["bash", script, "--root", root, ...args], {
    env: {
      PATH: process.env.PATH ?? "",
      FAKE_LOG: join(work, "calls.log"),
      FAKE_RULESET: join(work, "ruleset"),
      FAKE_SS: join(work, "ss"),
      ANIMA_ADMIN_USER: "admin",
      ...fakes,
      ...env,
    },
  });
  const log = join(work, "calls.log");
  const calls = existsSync(log) ? readFileSync(log, "utf8").trim().split("\n") : [];
  return { code: result.exitCode, out: result.stdout.toString() + result.stderr.toString(), calls };
}

const rootFile = (path: string): string => join(root, path);
const readRoot = (path: string): string => readFileSync(rootFile(path), "utf8");

function writeRoot(path: string, text: string): void {
  mkdirSync(join(rootFile(path), ".."), { recursive: true });
  writeFileSync(rootFile(path), text);
}

function listTree(dir: string): string[] {
  return readdirSync(dir, { recursive: true, encoding: "utf8" }).sort();
}

beforeEach(() => {
  work = mkdtempSync(join(tmpdir(), "anima-vps-"));
  root = join(work, "root");
  mkdirSync(join(work, "bin"), { recursive: true });
  for (const [name, body] of Object.entries(FAKES)) {
    const file = join(work, "bin", name);
    writeFileSync(file, `#!/bin/sh\nprintf '%s\\n' "${name} $*" >> "$FAKE_LOG"\n${body}\n`);
    chmodSync(file, 0o755);
  }
  writeFileSync(join(work, "ruleset"), RULESET);
  writeFileSync(join(work, "ss"), `${ALLOWED_SS.join("\n")}\n`);
  writeRoot("home/admin/.ssh/authorized_keys", TEST_KEY);
  writeRoot("etc/os-release", 'ID=ubuntu\nVERSION_ID="24.04"\n');
});

describe("nftables.conf", () => {
  const ruleset = parseNftables(readVps("nftables.conf"));
  const input = ruleset.chains.find((chain) => chain.hook === "input") as Chain;

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

  test.each([
    ["an unclosed table", "table inet t {\n chain input {\n"],
    ["a multi-line set", "table inet t {\n chain input {\n tcp dport {\n 22 } accept\n }\n}\n"],
    ["a stray brace", "}\n"],
  ])("VPS-002: the parser refuses %s instead of guessing", (_, text) => {
    expect(() => parseNftables(text)).toThrow(NftParseError);
  });

  test("VPS-002/DNS-005: VPN destinations not on tailscale0 are dropped before the public accepts", () => {
    const at = (rule: string): number => input.rules.indexOf(rule);
    const v4 = at('iifname != "tailscale0" ip daddr 100.64.0.0/10 drop');
    const v6 = at('iifname != "tailscale0" ip6 daddr fd7a:115c:a1e0::/48 drop');
    expect(v4).toBeGreaterThanOrEqual(0);
    expect(v6).toBeGreaterThanOrEqual(0);
    expect(Math.max(v4, v6)).toBeLessThan(at("tcp dport 443 accept"));
  });
});

describe("harden.sh ssh", () => {
  test.each([
    ["no authorized_keys", { ANIMA_ADMIN_USER: "nokeys" }],
    ["root", { ANIMA_ADMIN_USER: "root" }],
    ["a user not in group sudo", { FAKE_GROUPS: "admin users" }],
    ["a user that does not exist", { ANIMA_ADMIN_USER: "ghost" }],
    ["a name that could inject sshd config", { ANIMA_ADMIN_USER: "admin\nPermitRootLogin yes" }],
  ])("VPS-001: refuses %s", (_, env) => {
    const run = harden(["ssh"], env);
    expect(run.code).toBe(2);
    expect(existsSync(rootFile("etc/ssh/sshd_config.d/10-anima.conf"))).toBe(false);
  });

  test("VPS-001: refuses an authorized_keys with no key in it", () => {
    writeRoot("home/admin/.ssh/authorized_keys", "# no keys yet\n\n");
    expect(harden(["ssh"]).code).toBe(2);
  });

  test("VPS-001: installs the drop-in with AllowUsers, validates it and reloads ssh", () => {
    const run = harden(["ssh"]);
    expect(run.code).toBe(0);
    expect(readRoot("etc/ssh/sshd_config.d/10-anima.conf")).toContain("\nAllowUsers admin\n");
    expect(run.calls).toEqual(expect.arrayContaining(["SSHD -t", "SSHD -T", "SYSTEMCTL reload ssh"]));
    expect(run.out).not.toContain("AAAA");
  });

  test("VPS-001: removes the drop-in when the effective sshd values are wrong", () => {
    const run = harden(["ssh"], { FAKE_SSHD_PASSWORD: "yes" });
    expect(run.code).toBe(1);
    expect(existsSync(rootFile("etc/ssh/sshd_config.d/10-anima.conf"))).toBe(false);
    expect(run.calls).not.toContain("SYSTEMCTL reload ssh");
  });

  test("VPS-001: the drop-in disables password, keyboard-interactive and root login", () => {
    const lines = readVps("sshd-anima.conf").split("\n");
    for (const line of ["PasswordAuthentication no", "KbdInteractiveAuthentication no", "PermitRootLogin no"]) {
      expect(lines).toContain(line);
    }
    expect(lines).toContain("AuthenticationMethods publickey");
  });
});

describe("harden.sh firewall and confirm", () => {
  const vpn = { SSH_CONNECTION: "100.64.0.2 51000 100.64.0.1 22" };
  const applied = (run: Run): boolean => run.calls.some((call) => /^NFT -f /.test(call));

  test("VPS-002: firewall refuses without tailscale0", () => {
    const run = harden(["firewall"], { ...vpn, FAKE_TAILSCALE: "1" });
    expect(run.code).toBe(2);
    expect(applied(run)).toBe(false);
  });

  test.each([["203.0.113.9 51000 203.0.113.7 22"], [""]])(
    "VPS-002: firewall refuses a session that is not over the VPN (%p)",
    (connection) => {
      const run = harden(["firewall"], { SSH_CONNECTION: connection });
      expect(run.code).toBe(2);
      expect(applied(run)).toBe(false);
    },
  );

  test("VPS-002: firewall saves the ruleset, validates, arms a 180 s rollback, then applies", () => {
    const run = harden(["firewall"], vpn);
    expect(run.code).toBe(0);
    expect(readRoot("var/lib/anima/nftables.previous")).toBe(RULESET);
    const rollback = readRoot("var/lib/anima/nftables.rollback");
    expect(rollback).toContain("delete table inet anima_filter\ntable inet anima_filter {");
    expect(rollback).not.toContain("table ip nat");
    const index = (re: RegExp): number => run.calls.findIndex((call) => re.test(call));
    const check = index(/^NFT -c -f .*nftables\.conf$/);
    const arm = index(/^SYSTEMD_RUN --unit=anima-firewall-rollback --on-active=180 .* -f \/var\/lib\/anima\/nftables\.rollback$/);
    const apply = index(/^NFT -f /);
    expect(check).toBeGreaterThanOrEqual(0);
    expect(arm).toBeGreaterThan(check);
    expect(apply).toBeGreaterThan(arm);
    expect(existsSync(rootFile("etc/nftables.conf"))).toBe(false);
  });

  test("VPS-002: firewall applies nothing when nft -c fails", () => {
    const run = harden(["firewall"], { ...vpn, FAKE_NFT_CHECK: "1" });
    expect(run.code).toBe(1);
    expect(applied(run)).toBe(false);
    expect(run.calls.some((call) => call.startsWith("SYSTEMD_RUN"))).toBe(false);
  });

  test("VPS-002: confirm stops the rollback and installs the rules for boot", () => {
    expect(harden(["firewall"], vpn).code).toBe(0);
    const run = harden(["confirm"]);
    expect(run.code).toBe(0);
    expect(run.calls).toEqual(expect.arrayContaining(["SYSTEMCTL stop anima-firewall-rollback.timer"]));
    expect(run.calls).toContain("SYSTEMCTL enable nftables.service");
    expect(readRoot("etc/nftables.conf")).toBe(readVps("nftables.conf"));
  });

  test("VPS-002: confirm refuses once the rollback is no longer armed", () => {
    expect(harden(["firewall"], vpn).code).toBe(0);
    const run = harden(["confirm"], { FAKE_ACTIVE: "3" });
    expect(run.code).toBe(2);
    expect(existsSync(rootFile("etc/nftables.conf"))).toBe(false);
  });
});

describe("harden.sh updates and docker", () => {
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
    const run = harden(["updates"]);
    expect(run.code).toBe(0);
    expect(readRoot("etc/apt/apt.conf.d/52anima-unattended-upgrades")).toBe(upgrades);
    expect(run.calls).toContain("UNATTENDED_UPGRADE --dry-run");
  });

  test("D-10: daemon.json binds published ports to 127.0.0.1 by default", () => {
    expect(harden(["docker"]).code).toBe(0);
    expect(JSON.parse(readRoot("etc/docker/daemon.json"))).toMatchObject({ ip: "127.0.0.1", "live-restore": true });
  });

  test("D-10: merging keeps existing keys and forces the loopback bind", () => {
    writeRoot("etc/docker/daemon.json", '{"ip": "0.0.0.0", "data-root": "/srv/docker"}');
    expect(harden(["docker"]).code).toBe(0);
    const merged = JSON.parse(readRoot("etc/docker/daemon.json")) as Record<string, unknown>;
    expect(merged).toMatchObject({ ip: "127.0.0.1", "data-root": "/srv/docker" });
  });

  test.each([["{ broken"], ["[1]"], ['{"a": 1} {"b": 2}']])("D-10: refuses an existing daemon.json %p", (text) => {
    writeRoot("etc/docker/daemon.json", text);
    expect(harden(["docker"]).code).toBe(2);
    expect(readRoot("etc/docker/daemon.json")).toBe(text);
  });
});

describe("harden.sh check, dry run and lint", () => {
  test("VPS-002: check lists a public listener on 5432 as a violation", () => {
    writeFileSync(join(work, "ss"), `${ALLOWED_SS.join("\n")}\ntcp LISTEN 0 244 0.0.0.0:5432 0.0.0.0:*\n`);
    const run = harden(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: public listener tcp 0.0.0.0:5432");
    expect(run.out.match(/VIOLATION/g)).toHaveLength(1);
  });

  test("VPS-002: check passes with only allowed listeners", () => {
    const run = harden(["check"]);
    expect(run.out).toContain("ok: no violations");
    expect(run.code).toBe(0);
  });

  test("VPS-005: --dry-run writes nothing under the root", () => {
    writeRoot("var/lib/anima/nftables.pending", "pending\n");
    const before = listTree(root);
    const vpn = { SSH_CONNECTION: "100.64.0.2 51000 100.64.0.1 22" };
    for (const command of ["ssh", "updates", "docker", "firewall", "confirm"]) {
      const run = harden(["--dry-run", command], vpn);
      expect([command, run.code]).toEqual([command, 0]);
    }
    expect(listTree(root)).toEqual(before);
    expect(readRoot("var/lib/anima/nftables.pending")).toBe("pending\n");
  });

  test("CQ-024: harden.sh is shellcheck-clean", () => {
    const run = Bun.spawnSync(["shellcheck", script]);
    expect(run.stdout.toString()).toBe("");
    expect(run.exitCode).toBe(0);
  });
});
