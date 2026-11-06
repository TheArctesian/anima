import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { HARDEN_TIMEOUT_MS, fakeHost, removeHosts, type Host } from "./vps-host.ts";

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

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

  test("R-050: labels nothing and succeeds while a recorded deviation keeps SELinux disabled", () => {
    host.write("etc/anima/selinux-disabled", "# why\nR-050: the operator's services must not stop for a relabel\n");
    const run = host.run(["selinux"], { FAKE_ENFORCE: "Disabled" });
    expect(run.code).toBe(0);
    expect(run.out).toContain("a recorded deviation");
    expect(run.calls.some((call) => /^(SEMANAGE|RESTORECON)/.test(call))).toBe(false);
  });

  test("R-050: a deviation file with only comments records nothing", () => {
    host.write("etc/anima/selinux-disabled", "# no reason given\n\n");
    expect(host.run(["selinux"], { FAKE_ENFORCE: "Disabled" }).code).toBe(2);
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

  test("R-050: the operator's log settings win over the Anima defaults, which fill only what is missing", () => {
    host.write("etc/docker/daemon.json", '{"log-driver": "local", "log-opts": {"max-file": "3", "labels": "team"}}');
    expect(host.run(["docker"]).code).toBe(0);
    const merged = JSON.parse(host.read("etc/docker/daemon.json")) as Record<string, unknown>;
    expect(merged["log-driver"]).toBe("local");
    expect(merged["log-opts"]).toEqual({ "max-size": "10m", "max-file": "3", labels: "team" });
  });

  test("R-050: the operator's file cannot turn off the forced keys", () => {
    host.write("etc/docker/daemon.json", '{"ip": "0.0.0.0", "live-restore": false, "selinux-enabled": false, "userland-proxy": true, "default-address-pools": [{"base": "172.17.0.0/12", "size": 24}]}');
    expect(host.run(["docker"]).code).toBe(0);
    expect(JSON.parse(host.read("etc/docker/daemon.json"))).toEqual({
      ip: "127.0.0.1", "live-restore": true, "selinux-enabled": true, "userland-proxy": false, "log-driver": "json-file",
      "log-opts": { "max-size": "10m", "max-file": "5" }, "default-address-pools": [{ base: "172.17.0.0/12", size: 24 }],
    });
  });

  test("D-9: docker tells the operator to recreate Headscale, which keeps its old label", () => {
    expect(host.run(["--restart-docker", "docker"]).out).toContain("up -d --force-recreate");
  });

  test("D-10: docker restarts Docker only when asked", () => {
    expect(host.run(["docker"]).calls).not.toContain("SYSTEMCTL restart docker");
    expect(host.run(["--restart-docker", "docker"]).calls).toContain("SYSTEMCTL restart docker");
  });
});

describe("harden.sh docker --restart-docker (R-050)", () => {
  test("R-050: docker reloads Docker to turn live-restore on before it restarts it", () => {
    const run = host.run(["--restart-docker", "docker"]);
    expect(run.code).toBe(0);
    const reload = run.calls.indexOf("SYSTEMCTL reload docker");
    expect(reload).toBeGreaterThanOrEqual(0);
    expect(run.calls.indexOf("DOCKER info --format {{.LiveRestoreEnabled}}")).toBeGreaterThan(reload);
    expect(run.calls.indexOf("SYSTEMCTL restart docker")).toBeGreaterThan(reload);
    expect(run.out).toContain("every running container kept running");
  });

  test("R-050: docker does not restart Docker when live-restore is still off, which would stop every container", () => {
    const run = host.run(["--restart-docker", "docker"], { FAKE_LIVE_RESTORE: "false", HARDEN_LIVE_RESTORE_WAIT: "2" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("live-restore is not on");
    expect(run.calls).not.toContain("SYSTEMCTL restart docker");
  });

  test("R-050: docker does not restart Docker when docker info fails after the reload", () => {
    const run = host.run(["--restart-docker", "docker"], { FAKE_DOCKER_DOWN_INFO: "1" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("docker info failed after the reload");
    expect(run.calls).not.toContain("SYSTEMCTL restart docker");
  });

  test("R-050: docker fails naming a container that the restart stopped and started again", () => {
    const run = host.run(["--restart-docker", "docker"], { FAKE_IDS: "aaa111\\nbbb222\\n", FAKE_STARTED_AFTER_IDS: "bbb222" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("containers stopped by the restart: bbb222");
  });

  test("R-050: without --restart-docker, docker points to it instead of a plain restart", () => {
    expect(host.run(["docker"]).out).toContain("--restart-docker docker (a plain restart would stop every container)");
  });

  test("R-050: docker does not restart Docker when the reload fails", () => {
    const run = host.run(["--restart-docker", "docker"], { FAKE_RELOAD: "1" });
    expect(run.code).toBe(1);
    expect(run.calls).not.toContain("SYSTEMCTL restart docker");
  });

  test("R-050: docker fails and names each container the restart stopped", () => {
    const run = host.run(["--restart-docker", "docker"], { FAKE_IDS: "aaa111\\nbbb222\\n", FAKE_IDS_AFTER: "aaa111\\n" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("containers stopped by the restart: bbb222");
  });

  test("R-050: a dry run neither reloads nor restarts Docker", () => {
    const run = host.run(["--dry-run", "--restart-docker", "docker"]);
    expect(run.calls.some((call) => /^SYSTEMCTL (reload|restart) docker/.test(call))).toBe(false);
  });
});

describe("harden.sh check (Headscale's process label)", () => {
  beforeEach(() => {
    host.harden();
  });

  test("D-9: check passes when Headscale runs as container_t", () => {
    const run = host.run(["check"]);
    expect(run.calls).toContain(
      "DOCKER ps -q --filter label=com.docker.compose.project=headscale --filter label=com.docker.compose.service=headscale",
    );
    expect(run.calls).toContain("DOCKER inspect -f {{.ProcessLabel}} 0123abcd");
    expect(run.code).toBe(0);
  });

  test.each([
    ["spc_t, as before it is recreated", { FAKE_PROCESS_LABEL: "system_u:system_r:spc_t:s0" }, "Headscale runs as system_u:system_r:spc_t:s0, not container_t"],
    ["no label", { FAKE_PROCESS_LABEL: "" }, "Headscale runs as an unknown label, not container_t"],
    ["no running container", { FAKE_HEADSCALE_ID: "" }, "the Headscale container is not running"],
  ])("D-9: check reports %s", (_, env, message) => {
    const run = host.run(["check"], env);
    expect(run.code).toBe(1);
    expect(run.out).toContain(`VIOLATION: ${message}`);
  });

  test("R-050: check accepts disabled SELinux with a recorded deviation, and prints the reason", () => {
    host.write("etc/anima/selinux-disabled", "R-050: keep the operator's services running\n");
    const run = host.run(["check"], { FAKE_ENFORCE: "Disabled", FAKE_DOCKER_SECURITY: "[name=seccomp,profile=builtin]" });
    expect(run.code).toBe(0);
    expect(run.out).toContain("note: SELinux stays disabled, a recorded deviation: R-050: keep the operator's services running");
  });

  test("R-050: the SELinux deviation does not silence other violations", () => {
    host.write("etc/anima/selinux-disabled", "R-050\n");
    host.setListeners(['tcp LISTEN 0 511 0.0.0.0:80 0.0.0.0:* users:(("nginx",pid=1200,fd=6))']);
    const run = host.run(["check"], { FAKE_ENFORCE: "Disabled", FAKE_DOCKER_SECURITY: "[name=seccomp,profile=builtin]" });
    expect(run.code).toBe(1);
    expect(run.out).toMatch(/VIOLATION: public listener .*:80 /);
  });

  test("R-050: check reports a deviation file while SELinux is permissive, without the disabled note", () => {
    host.write("etc/anima/selinux-disabled", "R-050\n");
    const run = host.run(["check"], { FAKE_ENFORCE: "Permissive" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("records SELinux as disabled, but it is Permissive");
    expect(run.out).not.toContain("note: SELinux stays disabled");
  });

  test("R-050: check reports disabled SELinux without a deviation", () => {
    const run = host.run(["check"], { FAKE_ENFORCE: "Disabled" });
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: SELinux is Disabled, not Enforcing");
  });

  test("R-050: check reports a deviation file left behind once SELinux is on", () => {
    host.write("etc/anima/selinux-disabled", "R-050\n");
    const run = host.run(["check"]);
    expect(run.code).toBe(1);
    expect(run.out).toContain("VIOLATION: /etc/anima/selinux-disabled records SELinux as disabled, but it is Enforcing; remove the file");
  });

  test("D-9: check skips the process label when SELinux is disabled", () => {
    const run = host.run(["check"], { FAKE_ENFORCE: "Disabled", FAKE_PROCESS_LABEL: "" });
    expect(run.out).not.toContain("container_t");
  });
});
