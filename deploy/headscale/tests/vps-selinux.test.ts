import { beforeEach, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { fakeHost, type Host } from "./vps-host.ts";

// D-9: Headscale's bind mounts carry container_file_t, and Docker labels containers.
const SPEC = "/opt/anima/headscale/(config|lib)(/.*)?";
let host: Host;

beforeEach(() => {
  host = fakeHost();
});

describe("harden.sh selinux", () => {
  test("D-9: adds the file-context rule, then relabels the Headscale folder", () => {
    const run = host.run(["selinux"]);
    expect(run.code).toBe(0);
    const add = run.calls.indexOf(`SEMANAGE fcontext -a -t container_file_t ${SPEC}`);
    expect(add).toBeGreaterThanOrEqual(0);
    expect(run.calls.indexOf(`RESTORECON -R ${join(host.root, "opt/anima/headscale")}`)).toBeGreaterThan(add);
  });

  test("D-9: modifies the rule when it exists already, so a second run succeeds", () => {
    const run = host.run(["selinux"], { FAKE_FCONTEXT: `${SPEC}    all files    system_u:object_r:usr_t:s0\\n` });
    expect(run.code).toBe(0);
    expect(run.calls).toContain(`SEMANAGE fcontext -m -t container_file_t ${SPEC}`);
    expect(run.calls.some((call) => call.startsWith("SEMANAGE fcontext -a"))).toBe(false);
  });

  test("D-9: refuses while SELinux is disabled, and labels nothing", () => {
    const run = host.run(["selinux"], { FAKE_ENFORCE: "Disabled" });
    expect(run.code).toBe(2);
    expect(run.calls.some((call) => /^(SEMANAGE fcontext -[am]|RESTORECON)/.test(call))).toBe(false);
  });

  test("D-9: warns but labels when SELinux is permissive", () => {
    const run = host.run(["selinux"], { FAKE_ENFORCE: "Permissive" });
    expect(run.code).toBe(0);
    expect(run.out).toContain("warning: SELinux is Permissive");
  });

  test("D-9: refuses before Headscale's folders exist", () => {
    rmSync(join(host.root, "opt/anima/headscale/lib"), { recursive: true });
    expect(host.run(["selinux"]).code).toBe(2);
  });
});

describe("harden.sh docker", () => {
  test("D-10: docker installs daemon.json with the loopback bind and SELinux labels", () => {
    expect(host.run(["docker"]).code).toBe(0);
    expect(JSON.parse(host.read("etc/docker/daemon.json"))).toMatchObject({ ip: "127.0.0.1", "selinux-enabled": true });
  });

  test("D-9: docker refuses while Headscale's volumes are not labelled container_file_t", () => {
    const run = host.run(["docker"], { FAKE_LABEL: "unconfined_u:object_r:usr_t:s0" });
    expect(run.code).toBe(2);
    expect(run.out).toContain("run harden.sh selinux first");
    expect(host.exists("etc/docker/daemon.json")).toBe(false);
  });

  test("D-9: docker does not need labels when SELinux is disabled", () => {
    expect(host.run(["docker"], { FAKE_ENFORCE: "Disabled", FAKE_LABEL: "?" }).code).toBe(0);
  });

  test("D-10: merging keeps existing keys and forces the loopback bind", () => {
    host.write("etc/docker/daemon.json", '{"ip": "0.0.0.0", "data-root": "/srv/docker"}');
    expect(host.run(["docker"]).code).toBe(0);
    const merged = JSON.parse(host.read("etc/docker/daemon.json")) as Record<string, unknown>;
    expect(merged).toMatchObject({ ip: "127.0.0.1", "data-root": "/srv/docker" });
    expect(host.read("var/lib/anima/daemon.json.previous")).toBe('{"ip": "0.0.0.0", "data-root": "/srv/docker"}');
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
