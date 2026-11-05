/**
 * A fake VPS for the harden.sh tests (D-14): a temporary root, a copy of `vps/`, and fake
 * system commands that log every call and act on FAKE_* variables.
 */
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

/**
 * Per-test timeout for the harden.sh tests. Each test runs harden.sh in bash with a dozen
 * fake commands; `check` alone starts some hundred processes, which takes more than bun's
 * 5 s default on a loaded machine. bun applies setDefaultTimeout per test file.
 */
export const HARDEN_TIMEOUT_MS = 30_000;

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

// `firewall-cmd --info-policy` of a running firewalld 2.4.3 on AlmaLinux 10: " (active)"
// after the name, trailing blanks kept.
const KEYS = "  services: \n  ports: \n  protocols: \n  masquerade: no\n  forward-ports: \n  source-ports: \n  icmp-blocks: \n  rich rules: \n";
const ICMP = ["mld-listener-done", "mld-listener-query", "mld-listener-report", "mld2-listener-report",
  "neighbour-advertisement", "neighbour-solicitation", "redirect", "router-advertisement"];
const POLICIES: Record<string, string> = {
  "allow-host-ipv6": "allow-host-ipv6 (active)\n  disable: no\n  priority: -15000\n  target: CONTINUE\n  ingress-zones: ANY\n" +
    `  egress-zones: HOST\n${KEYS}${ICMP.map((name) => `\trule family="ipv6" icmp-type name="${name}" accept\n`).join("")}`,
  "docker-forwarding": `docker-forwarding (active)\n  disable: no\n  priority: -1\n  target: ACCEPT\n  ingress-zones: ANY\n  egress-zones: docker\n${KEYS}`,
};

// `systemctl show -p ... nftables.service` on AlmaLinux 10 (systemd 257), stock and with the
// Anima drop-in; systemd prints no line for an empty command list.
const exec = (name: string, argv: string): string =>
  `${name}={ path=/sbin/nft ; argv[]=${argv} ; ignore_errors=no ; start_time=[n/a] ; stop_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }\n`;
const START = exec("ExecStart", "/sbin/nft -f /etc/sysconfig/nftables.conf");
const FRAGMENT = "FragmentPath=/usr/lib/systemd/system/nftables.service\n";
const NFT_UNIT = {
  stock: `${START}${exec("ExecReload", '/sbin/nft flush ruleset; include "/etc/sysconfig/nftables.conf";')}` +
    `${exec("ExecStop", "/sbin/nft flush ruleset")}${FRAGMENT}DropInPaths=\n`,
  anima: `${START}${exec("ExecReload", "/usr/sbin/nft -f /etc/sysconfig/nftables.conf").replace("path=/sbin", "path=/usr/sbin")}` +
    `${FRAGMENT}DropInPaths=/etc/systemd/system/nftables.service.d/anima.conf\n`,
};

// `ip -o -4 addr show` on a VPS with Docker and the VPN: loopback, public, docker0, tailnet.
const ADDRS = [
  "1: lo    inet 127.0.0.1/8 scope host lo",
  "2: eth0    inet 203.0.113.7/24 brd 203.0.113.255 scope global eth0",
  "3: docker0    inet 172.17.0.1/16 brd 172.17.255.255 scope global docker0",
  "4: br-5b1c2d3e4f60    inet 172.30.0.1/24 brd 172.30.0.255 scope global br-5b1c2d3e4f60",
  "5: tailscale0    inet 100.64.0.1/32 scope global tailscale0",
].join("\\n");

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
  is-enabled) [ "$unit" = firewalld.service ] && { echo "\${FAKE_FIREWALLD:-masked}"; exit 1; }
              case "$unit" in ip*tables.service) [ "\${FAKE_IPTABLES:-disabled}" = enabled ]; exit $?;; esac; exit "\${FAKE_ENABLED:-0}";;
  list-unit-files) printf '%b\\n' "\${FAKE_UNITS:-sshd.service enabled enabled}";;
  show) [ "$unit" = nftables.service ] || { echo "\${FAKE_SUBSTATE:-waiting}"; exit 0; }
        f=stock; [ ! -f "$FAKE_ROOT/etc/systemd/system/nftables.service.d/anima.conf" ] || f=anima
        sed "\${FAKE_NFT_UNIT_EDIT:-}" "$FAKE_DIR/nft-unit-$f"; printf '%b' "\${FAKE_NFT_UNIT_EXTRA:-}";; reload) exit "\${FAKE_RELOAD:-0}";; restart) [ "$unit" != docker ] || : > "$FAKE_DIR/docker-restarted"; exit "\${FAKE_RESTART:-0}";; mask) exit "\${FAKE_MASK:-0}";;
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
  --info-policy=*) f="$FAKE_DIR/policy-\${1#*=}"; [ -f "$f" ] || exit 1
    sed "\${FAKE_FW_POLICY_EDIT:-}" "$f"; printf '%b' "\${FAKE_FW_POLICY_EXTRA:-}";;
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
  IP: `[ "$1" = -o ] || exit "\${FAKE_TAILSCALE:-0}"
printf '%b\\n' "\${FAKE_ADDRS:-${ADDRS}}"`,
  ID: `echo "\${FAKE_GROUPS:-$2 wheel}"`,
  GETENT: `case "$2" in
  ghost) exit 2;;
  root | toor) echo "$2:x:0:0::/root:/bin/sh";;
  relhome) echo "$2:x:$FAKE_UID:1000::home/$2:/bin/sh";;
  *) echo "$2:x:\${FAKE_OWNER:-$FAKE_UID}:1000::/home/$2:/bin/bash";;
esac`,
  DOCKER: `case "$1" in
  version) [ -z "\${FAKE_DOCKER_DOWN:-}" ] || exit 1; echo "\${FAKE_DOCKER_VERSION:-29.9.3}";;
  info) case "$*" in *LiveRestoreEnabled*) [ -z "\${FAKE_DOCKER_DOWN_INFO:-}" ] || exit 1; echo "\${FAKE_LIVE_RESTORE:-true}";;
          *) echo "\${FAKE_DOCKER_SECURITY:-[name=seccomp,profile=builtin name=selinux]}";; esac;;
  ps) if [ "$2" = -q ] && [ "$3" = --no-trunc ]; then
        if [ -f "$FAKE_DIR/docker-restarted" ]; then printf '%b' "\${FAKE_IDS_AFTER-\${FAKE_IDS:-aaa\\nbbb\\n}}"; else printf '%b' "\${FAKE_IDS:-aaa\\nbbb\\n}"; fi
        exit 0
      fi
      [ "$2" != -q ] || { printf '%b' "\${FAKE_HEADSCALE_ID-0123abcd\\n}"; exit 0; }
      printf '%b\\n' "\${FAKE_CONTAINERS:-headscale-headscale-1 docker.io/headscale/headscale:0.29.7 }";;
  inspect) case "$3" in *StartedAt*) shift 3
        for id; do t=\${FAKE_STARTED:-2026-11-01T00:00:00Z}
          if [ -f "$FAKE_DIR/docker-restarted" ]; then case " \${FAKE_STARTED_AFTER_IDS:-} " in *" $id "*) t=2026-11-05T00:00:00Z;; esac; fi
          echo "$id $t"; done;;
      *) echo "\${FAKE_PROCESS_LABEL-system_u:system_r:container_t:s0:c241,c692}";; esac;;
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

/** Write the fake system commands and the outputs they replay into the work folder. */
function installFakes(work: string): void {
  mkdirSync(join(work, "bin"));
  for (const [name, body] of Object.entries(FAKES)) {
    const file = join(work, "bin", name);
    writeFileSync(file, `#!/bin/sh\nprintf '%s\\n' "${name} $*" >> "$FAKE_LOG"\n${body}\n`);
    chmodSync(file, 0o755);
  }
  writeFileSync(join(work, "ruleset"), RULESET);
  for (const [name, text] of Object.entries(POLICIES)) writeFileSync(join(work, `policy-${name}`), text);
  for (const [name, text] of Object.entries(NFT_UNIT)) writeFileSync(join(work, `nft-unit-${name}`), text);
}

/** Run the copy of harden.sh in `work` against the fakes and the temporary root. */
function runHarden(work: string, args: string[], env: Record<string, string>): Run {
  const log = join(work, "calls.log");
  const root = join(work, "root");
  writeFileSync(log, "");
  const fakes = Object.keys(FAKES).map((name) => [`HARDEN_${name}`, join(work, "bin", name)]);
  const result = Bun.spawnSync(["bash", join(work, "vps", "harden.sh"), "--root", root, ...args], {
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
}

export function fakeHost(): Host {
  const work = mkdtempSync(join(tmpdir(), "anima-vps-"));
  const root = join(work, "root");
  const vps = join(work, "vps");
  cpSync(VPS_DIR, vps, { recursive: true });
  installFakes(work);
  const host: Host = {
    root,
    vps,
    run: (args, env = {}) => runHarden(work, args, env),
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
