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

// sshd -T: first value wins across the drop-ins in name order; AllowUsers adds up.
const SSHD_DEFAULTS =
  "passwordauthentication yes kbdinteractiveauthentication yes permitrootlogin prohibit-password " +
  "permitemptypasswords no pubkeyauthentication yes authenticationmethods any x11forwarding no " +
  "maxauthtries 6 logingracetime 120";
const SSHD_T = `awk 'BEGIN { n = split("${SSHD_DEFAULTS}", d, " "); for (i = 1; i < n; i += 2) dflt[d[i]] = d[i + 1] }
/^[[:space:]]*(#|$)/ { next }
{ k = tolower($1); if (k == "allowusers") { for (i = 2; i <= NF; i++) print "allowusers " $i; next }; if (!(k in v)) v[k] = $2 }
END { for (k in dflt) print k " " ((k in v) ? v[k] : dflt[k]) }' "$@" </dev/null`;

const FAKES: Record<string, string> = {
  NFT: `case "$1" in
  -c) case "$3" in *rollback) exit "\${FAKE_NFT_CHECK_ROLLBACK:-0}";; *) exit "\${FAKE_NFT_CHECK:-0}";; esac;;
  -f) exit "\${FAKE_NFT_APPLY:-0}";;
  list) [ "$2" = ruleset ] && cat "$FAKE_RULESET" && exit 0; exit "\${FAKE_NFT_TABLE:-0}";;
esac`,
  SYSTEMCTL: `case "$1" in
  is-active) exit "\${FAKE_ACTIVE:-0}";; is-enabled) exit "\${FAKE_ENABLED:-0}";;
  show) echo "\${FAKE_SUBSTATE:-waiting}";; reload) exit "\${FAKE_RELOAD:-0}";;
  stop) [ -z "\${FAKE_STOP_STARTS_ROLLBACK:-}" ] || : > "$FAKE_ROOT/var/lib/anima/firewall-rolled-back";;
esac`,
  SYSTEMD_RUN: "exit 0",
  SSHD: `[ "$1" = -T ] || exit "\${FAKE_SSHD_SYNTAX:-0}"
set -- "$FAKE_ROOT"/etc/ssh/sshd_config.d/*.conf
[ -e "$1" ] || set --
${SSHD_T}`,
  SS: `cat "$FAKE_SS"`,
  APT_GET: "exit 0",
  UNATTENDED_UPGRADE: "exit 0",
  IP: `exit "\${FAKE_TAILSCALE:-0}"`,
  ID: `echo "\${FAKE_GROUPS:-$2 sudo}"`,
  GETENT: `case "$2" in
  ghost) exit 2;;
  root | toor) echo "$2:x:0:0::/root:/bin/sh";;
  relhome) echo "$2:x:$FAKE_UID:1000::home/$2:/bin/sh";;
  *) echo "$2:x:\${FAKE_OWNER:-$FAKE_UID}:1000::/home/$2:/bin/bash";;
esac`,
  DOCKER: `echo "\${FAKE_DOCKER_VERSION:-28.5.1}"`,
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
  /** Install what `ssh` and `confirm` install, so `check` sees a hardened host. */
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
    harden() {
      const dropIn = readFileSync(join(vps, "sshd-anima.conf"), "utf8").replace("${ANIMA_ADMIN_USER}", "admin");
      host.write("etc/ssh/sshd_config.d/10-anima.conf", dropIn);
      host.write("etc/systemd/system/nftables.service.d/anima.conf", "[Service]\n");
    },
  };
  host.write("home/admin/.ssh/authorized_keys", TEST_KEY, 0o600);
  chmodSync(join(root, "home/admin/.ssh"), 0o700);
  chmodSync(join(root, "home/admin"), 0o755);
  host.write("etc/os-release", 'ID=ubuntu\nVERSION_ID="24.04"\n');
  host.setListeners(ALLOWED_SS);
  return host;
}

/** Mutating commands: a dry run must call none of these. */
export function mutatingCalls(run: Run): string[] {
  return run.calls.filter(
    (call) =>
      /^(APT_GET|SYSTEMD_RUN|UNATTENDED_UPGRADE) /.test(call) ||
      /^NFT -f /.test(call) ||
      (/^SYSTEMCTL /.test(call) && !/^SYSTEMCTL (is-active|is-enabled|show) /.test(call)),
  );
}
