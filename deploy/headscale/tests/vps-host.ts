/**
 * A fake VPS for the harden.sh tests (D-14): a temporary root, a copy of `vps/`, and fake
 * system commands that log every call and act on FAKE_* variables.
 */
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

export const VPS_DIR = join(import.meta.dir, "..", "vps");
export const VPN_SESSION = "100.64.0.2 51000 100.64.0.1 22";
export const ALLOWED_SS = [
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
export const RULESET = "table ip nat {\n}\ntable inet anima_filter {\n\tchain input {\n\t}\n}\n";
const TEST_KEY = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestOnlyNotARealKey admin@test\n";

// The stock AlmaLinux 10 sshd_config, cut to the lines that matter here.
const SSHD_CONFIG =
  "# stock comments\nInclude /etc/ssh/sshd_config.d/*.conf\nAuthorizedKeysFile .ssh/authorized_keys\n" +
  "#Match User anoncvs\n#\tPermitTTY no\nSubsystem sftp /usr/libexec/openssh/sftp-server\n";

const FAKES: Record<string, string> = {
  NFT: `case "$1" in
  -c) if [ "$3" = /dev/stdin ]; then
        cat > "$FAKE_DIR/stdin.nft"
        if grep -q '^include ' "$FAKE_DIR/stdin.nft"; then mv "$FAKE_DIR/stdin.nft" "$FAKE_DIR/boot.nft"; exit "\${FAKE_NFT_CHECK_BOOT:-0}"; fi
        mv "$FAKE_DIR/stdin.nft" "$FAKE_DIR/checked.nft"
      fi
      case "$3" in *rollback) exit "\${FAKE_NFT_CHECK_ROLLBACK:-0}";; */etc/sysconfig/nftables.conf) exit "\${FAKE_NFT_CHECK_BOOT:-0}";; *) exit "\${FAKE_NFT_CHECK:-0}";; esac;;
  -f) exit "\${FAKE_NFT_APPLY:-0}";;
  list) [ "$2" = ruleset ] && cat "$FAKE_RULESET" && exit 0
        [ "$2" = tables ] && echo "table ip nat" && exit 0; exit "\${FAKE_NFT_TABLE:-0}";;
esac`,
  SYSTEMCTL: `unit=$(printf '%s\\n' "$@" | tail -n 1)
case "$1" in
  is-active) case "$unit" in firewalld.service) exit "\${FAKE_FIREWALLD_ACTIVE:-3}";; *) exit "\${FAKE_ACTIVE:-0}";; esac;;
  is-enabled) [ "$unit" = firewalld.service ] && { echo "\${FAKE_FIREWALLD:-masked}"; exit 1; }; exit "\${FAKE_ENABLED:-0}";;
  list-unit-files) printf '%b\\n' "\${FAKE_UNITS:-sshd.service enabled enabled}";;
  show) echo "\${FAKE_SUBSTATE:-waiting}";; reload) exit "\${FAKE_RELOAD:-0}";; mask) exit "\${FAKE_MASK:-0}";;
  stop) [ -z "\${FAKE_STOP_STARTS_ROLLBACK:-}" ] || : > "$FAKE_ROOT/var/lib/anima/firewall-rolled-back";;
esac`,
  SYSTEMD_RUN: `[ "$1" != --wait ] || exit "\${FAKE_PROBE:-0}"`,
  SSHD: readFileSync(join(import.meta.dir, "fake-sshd.sh"), "utf8"),
  SS: `case "$*" in *established*) printf '%b' "\${FAKE_SS_ESTABLISHED:-}";; *) cat "$FAKE_SS";; esac`,
  DNF: "exit 0",
  FIREWALL_CMD: `tabbed() { [ -z "$1" ] || printf '\\t%s\\n' "$1"; }
case "$1" in
  --get-active-zones) printf '%b\\n' "\${FAKE_ZONES:-docker\\n  interfaces: docker0\\npublic (default)}";;
  --get-active-policies) printf '%b\\n' "\${FAKE_FW_POLICIES:-allow-host-ipv6\\n  ingress-zones: ANY\\n  egress-zones: HOST}";;
  --direct) [ "$2" != --get-all-rules ] || printf '%b' "\${FAKE_FW_DIRECT:-}";;
  --info-service=*) i=; case "$1" in *=cockpit) p=9090/tcp;; *=ssh) p=22/tcp;; *=dhcpv6-client) p=546/udp;; *=bundle) p=; i=ssh;; *) p=;; esac
                    printf '%s\\n  ports: %s\\n  protocols: \\n  source-ports: \\n  includes: %s\\n' "\${1#*=}" "$p" "$i";;
  --zone=*) [ "$2" = --list-all ] || exit 0
    printf '%s (active)\\n  target: %s\\n  ingress-priority: 0\\n  icmp-block-inversion: no\\n  interfaces: eth0\\n' \\
      "\${1#--zone=}" "\${FAKE_FW_TARGET:-default}"
    printf '  sources: %s\\n  services: %s\\n  ports: %s\\n  protocols: %s\\n  forward: yes\\n  masquerade: %s\\n' \\
      "\${FAKE_FW_SOURCES:-}" "\${FAKE_FW_SERVICES:-ssh}" "\${FAKE_FW_PORTS:-}" "\${FAKE_FW_PROTOCOLS:-}" "\${FAKE_FW_MASQ:-no}"
    echo '  forward-ports: '; tabbed "\${FAKE_FW_FORWARD_PORTS:-}"
    printf '  source-ports: %s\\n  icmp-blocks: \\n  rich rules: \\n' "\${FAKE_FW_SOURCE_PORTS:-}"; tabbed "\${FAKE_FW_RICH:-}";;
esac`,
  SEMANAGE: `[ "$2" != -l ] || printf '%b' "\${FAKE_FCONTEXT:-}"`,
  RESTORECON: "exit 0",
  GETENFORCE: `echo "\${FAKE_ENFORCE:-Enforcing}"`,
  STAT: `[ -e "$3" ] || exit 1; echo "\${FAKE_LABEL:-system_u:object_r:container_file_t:s0}"`,
  PS: `echo "\${FAKE_UNIT:-sshd.service}"`,
  IP: `exit "\${FAKE_TAILSCALE:-0}"`,
  ID: `echo "\${FAKE_GROUPS:-$2 wheel}"`,
  GETENT: `case "$2" in
  ghost) exit 2;;
  root | toor) echo "$2:x:0:0::/root:/bin/sh";;
  relhome) echo "$2:x:$FAKE_UID:1000::home/$2:/bin/sh";;
  *) echo "$2:x:\${FAKE_OWNER:-$FAKE_UID}:1000::/home/$2:/bin/bash";;
esac`,
  DOCKER: `case "$1" in
  version) echo "\${FAKE_DOCKER_VERSION:-29.9.3}";;
  info) echo "\${FAKE_DOCKER_SECURITY:-[name=seccomp,profile=builtin name=selinux]}";;
  ps) [ "$2" != -q ] || { printf '%b' "\${FAKE_HEADSCALE_ID-0123abcd\\n}"; exit 0; }
      printf '%b\\n' "\${FAKE_CONTAINERS:-headscale-headscale-1 docker.io/headscale/headscale:0.29.7 }";;
  inspect) echo "\${FAKE_PROCESS_LABEL-system_u:system_r:container_t:s0:c241,c692}";;
  network) case "$2" in
      ls) printf '%b\\n' "\${FAKE_NETWORKS:-bridge bridge 2673f396533215b1e304\\nhost host 3779e81102d984fcc41a}";;
      inspect) case "$5" in bridge) echo docker0;; *) echo "\${FAKE_BRIDGE:-<no value>}";; esac;;
    esac;;
esac`,
};

export interface Run {
  code: number;
  out: string;
  calls: string[];
}

export interface Host {
  root: string;
  /** The copy of `vps/` the script runs from. */
  vps: string;
  run(args: string[], env?: Record<string, string>): Run;
  write(path: string, text: string, mode?: number): void;
  read(path: string): string;
  exists(path: string): boolean;
  tree(): string[];
  setListeners(lines: string[]): void;
  /** The rules `firewall` last passed to `nft -c` (with the local accepts in place). */
  checkedRules(): string;
  /** Install what `ssh`, `updates`, `selinux` and `confirm` install, so `check` sees a hardened host. */
  harden(): void;
}

export function fakeHost(): Host {
  const work = mkdtempSync(join(tmpdir(), "anima-vps-"));
  const root = join(work, "root");
  const vps = join(work, "vps");
  cpSync(VPS_DIR, vps, { recursive: true });
  mkdirSync(join(work, "bin"));
  for (const [name, body] of Object.entries(FAKES)) {
    const file = join(work, "bin", name);
    writeFileSync(file, `#!/bin/sh\nprintf '%s\\n' "${name} $*" >> "$FAKE_LOG"\n${body}\n`);
    chmodSync(file, 0o755);
  }
  writeFileSync(join(work, "ruleset"), RULESET);
  const host: Host = {
    root,
    vps,
    run(args, env = {}) {
      const log = join(work, "calls.log");
      writeFileSync(log, "");
      const fakes = Object.keys(FAKES).map((name) => [`HARDEN_${name}`, join(work, "bin", name)]);
      const result = Bun.spawnSync(["bash", join(vps, "harden.sh"), "--root", root, ...args], {
        env: {
          PATH: process.env.PATH ?? "",
          FAKE_LOG: log,
          FAKE_RULESET: join(work, "ruleset"),
          FAKE_SS: join(work, "ss"),
          FAKE_ROOT: root,
          FAKE_DIR: work,
          FAKE_UID: String(process.getuid?.() ?? 0),
          ANIMA_ADMIN_USER: "admin",
          ...Object.fromEntries(fakes),
          ...env,
        },
      });
      const calls = readFileSync(log, "utf8").split("\n").filter(Boolean);
      return { code: result.exitCode, out: result.stdout.toString() + result.stderr.toString(), calls };
    },
    write(path, text, mode = 0o644) {
      mkdirSync(dirname(join(root, path)), { recursive: true });
      writeFileSync(join(root, path), text);
      chmodSync(join(root, path), mode);
    },
    read: (path) => readFileSync(join(root, path), "utf8"),
    exists: (path) => existsSync(join(root, path)),
    tree: () => readdirSync(root, { recursive: true, encoding: "utf8" }).sort(),
    setListeners: (lines) => { writeFileSync(join(work, "ss"), `${lines.join("\n")}\n`); },
    checkedRules: () => readFileSync(join(work, "checked.nft"), "utf8"),
    harden() {
      const dropIn = readFileSync(join(vps, "sshd-anima.conf"), "utf8").replace("${ANIMA_ADMIN_USER}", "admin");
      host.write("etc/ssh/sshd_config.d/00-anima.conf", dropIn);
      host.write("etc/systemd/system/nftables.service.d/anima.conf", "[Service]\n");
      host.write("etc/sysconfig/nftables.conf", '# stock comments\ninclude "/etc/nftables/anima.nft"\n');
      host.write("etc/dnf/automatic.conf", readFileSync(join(vps, "dnf-automatic.conf"), "utf8"));
      host.write("etc/systemd/system/dnf-automatic.timer.d/anima.conf", "[Timer]\n");
    },
  };
  host.write("home/admin/.ssh/authorized_keys", TEST_KEY, 0o600);
  chmodSync(join(root, "home/admin/.ssh"), 0o700);
  chmodSync(join(root, "home/admin"), 0o755);
  host.write("etc/ssh/sshd_config", SSHD_CONFIG);
  host.write("etc/os-release", 'ID="almalinux"\nVERSION_ID="10.2"\n');
  mkdirSync(join(root, "opt/anima/headscale/config"), { recursive: true });
  mkdirSync(join(root, "opt/anima/headscale/lib"), { recursive: true });
  host.setListeners(ALLOWED_SS);
  return host;
}

/** Mutating commands: a dry run must call none of these. */
export function mutatingCalls(run: Run): string[] {
  return run.calls.filter(
    (call) =>
      /^(DNF|SYSTEMD_RUN|RESTORECON) /.test(call) ||
      /^SEMANAGE fcontext -[am] /.test(call) ||
      /^NFT -f /.test(call) ||
      (/^SYSTEMCTL /.test(call) && !/^SYSTEMCTL (is-active|is-enabled|show|list-unit-files) /.test(call)),
  );
}
