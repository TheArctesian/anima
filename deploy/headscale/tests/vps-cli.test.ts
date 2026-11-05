import { afterEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, fakeHost, removeHosts, mutatingCalls, VPN_SESSION, VPS_DIR } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

describe("harden.sh dry run, usage and lint", () => {
  test("VPS-005: --dry-run writes nothing and calls no mutating command", () => {
    const host = fakeHost();
    host.write("var/lib/anima/nftables.pending", "pending\n");
    host.write("var/lib/anima/firewall-session", "100.64.0.2 50000 100.64.0.1 22\n");
    host.write("var/lib/anima/firewall-peers", "100.64.0.2 50000\n");
    const before = host.tree();
    for (const command of ["ssh", "updates", "selinux", "docker", "firewall", "confirm", "check"]) {
      const run = host.run(["--dry-run", "--restart-docker", command], { SSH_CONNECTION: VPN_SESSION });
      expect([command, mutatingCalls(run)]).toEqual([command, []]);
      expect([command, run.code]).toEqual([command, command === "check" ? 1 : 0]);
    }
    expect(host.tree()).toEqual(before);
    expect(host.read("var/lib/anima/nftables.pending")).toBe("pending\n");
  });

  test.each([[[]], [["bogus"]], [["--bogus", "check"]], [["check", "ssh"]], [["--root"]], [["--allow-closing"]]])(
    "usage errors exit 64: %p",
    (args) => {
      expect(fakeHost().run(args).code).toBe(64);
    },
  );

  test("--help prints the exit codes and exits 0", () => {
    const run = fakeHost().run(["--help"]);
    expect(run.code).toBe(0);
    expect(run.out).toContain("Exit codes: 0 done; 1");
  });

  test("CQ-024: harden.sh and the files it sources are shellcheck-clean", () => {
    const files = ["harden.sh", "sshd.sh", "selinux.sh", "open-ports.sh", "firewall.sh", "inventory.sh"].map((name) => join(VPS_DIR, name));
    const run = Bun.spawnSync(["shellcheck", "-x", ...files]);
    expect(run.stdout.toString()).toBe("");
    expect(run.exitCode).toBe(0);
  });
});
