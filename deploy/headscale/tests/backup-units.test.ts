import { describe, expect, test } from "bun:test";
import { unitValues } from "./backup-fixture.ts";

// VPS-006 / D-12: the systemd units that run the backup (VPS) and receive and prune it (home PC).

const UNITS = ["headscale-backup.unit", "rest-server.unit", "restic-prune.unit"];

/** The sandbox every backup unit shares. */
function expectSandboxed(file: string): void {
  const value = (key: string) => unitValues(file, "Service", key);
  for (const key of ["NoNewPrivileges", "ProtectHome", "PrivateTmp", "PrivateDevices", "MemoryDenyWriteExecute"]) {
    expect({ file, key, value: value(key) }).toEqual({ file, key, value: ["yes"] });
  }
  expect(value("ProtectSystem")).toEqual(["strict"]);
  expect(value("CapabilityBoundingSet")).toEqual([""]);
  expect(value("AmbientCapabilities")).toEqual([""]);
  expect(value("SystemCallFilter")).toEqual(["@system-service", "~@privileged"]);
  expect(value("IPAddressDeny")).toEqual(["any"]);
  expect(value("UMask")).toEqual(["0077"]);
}

describe("systemd units", () => {
  test.each(UNITS)("VPS-006 %s is sandboxed, without capabilities and denied every address by default", (file) => {
    expectSandboxed(file);
  });

  test("VPS-006 the backup timer runs daily at 03:15 UTC with a random delay and catches up missed runs", () => {
    expect(unitValues("headscale-backup.timer", "Timer", "OnCalendar")).toEqual(["*-*-* 03:15:00 UTC"]);
    expect(unitValues("headscale-backup.timer", "Timer", "RandomizedDelaySec")).toEqual(["15m"]);
    expect(unitValues("headscale-backup.timer", "Timer", "Persistent")).toEqual(["true"]);
    expect(unitValues("headscale-backup.timer", "Timer", "Unit")).toEqual(["headscale-backup.service"]);
  });

  test("VPS-006 the backup unit is a oneshot that reads Headscale read-only and its secrets with LoadCredential", () => {
    const value = (key: string) => unitValues("headscale-backup.unit", "Service", key);
    expect(value("Type")).toEqual(["oneshot"]);
    expect(value("ReadOnlyPaths")).toEqual(["/opt/anima/headscale"]);
    expect(value("ReadWritePaths")).toEqual([]);
    expect(value("IPAddressAllow")).toEqual(["10.10.0.0/24"]);
    expect(value("LimitCORE")).toEqual(["0"]);
    expect(value("LoadCredential").map((entry) => entry.split(":")[0])).toEqual(["restic-repository", "restic-password"]);
    expect(value("Environment")).toEqual(expect.arrayContaining([
      "RESTIC_REPOSITORY_FILE=%d/restic-repository", "RESTIC_PASSWORD_FILE=%d/restic-password",
    ]));
    expect(value("Environment").filter((entry) => /^RESTIC_(PASSWORD|REPOSITORY)=/.test(entry))).toEqual([]);
  });

  test("VPS-006 rest-server is append-only with private repositories, authenticated, size-capped, as user restic", () => {
    const value = (key: string) => unitValues("rest-server.unit", "Service", key);
    const argv = value("ExecStart")[0]?.split(/\s+/) ?? [];
    expect(argv).toEqual(expect.arrayContaining(["--append-only", "--private-repos", "--path", "/srv/restic"]));
    expect(argv).toEqual(expect.arrayContaining(["--htpasswd-file", "/etc/restic/htpasswd", "--max-size", "1073741824"]));
    expect(argv).toEqual(expect.arrayContaining(["--listen", "${HOME_PC_LAN_IP}:8000"]));
    expect(argv).not.toContain("--no-auth");
    expect(value("ExecStartPre")).toEqual(["/usr/bin/test -n ${HOME_PC_LAN_IP}"]);
    expect(value("RestartSec")).toEqual(["10"]);
    expect(unitValues("rest-server.unit", "Unit", "StartLimitIntervalSec")).toEqual(["300"]);
    expect(unitValues("rest-server.unit", "Unit", "StartLimitBurst")).toEqual(["5"]);
    expect(value("User")).toEqual(["restic"]);
    expect(value("ReadWritePaths")).toEqual(["/srv/restic"]);
  });

  test("VPS-006 only the hub's VPN address may reach rest-server, through the operator's drop-in", () => {
    expect(unitValues("rest-server.unit", "Service", "IPAddressAllow")).toEqual([]);
    expect(unitValues("rest-server-allow.conf", "Service", "IPAddressAllow")).toEqual(["${HUB_VPN_IPV4}"]);
  });

  test("VPS-006 prune runs weekly on the home PC, offline, as user restic, with its password as a credential", () => {
    const value = (key: string) => unitValues("restic-prune.unit", "Service", key);
    expect(unitValues("restic-prune.timer", "Timer", "OnCalendar")).toEqual(["Sun *-*-* 05:00:00"]);
    expect(unitValues("restic-prune.timer", "Timer", "Persistent")).toEqual(["true"]);
    expect(unitValues("restic-prune.timer", "Timer", "Unit")).toEqual(["restic-prune.service"]);
    expect(value("User")).toEqual(["restic"]);
    expect(value("PrivateNetwork")).toEqual(["yes"]);
    expect(value("LimitCORE")).toEqual(["0"]);
    expect(value("IPAddressAllow")).toEqual([]);
    expect(value("ReadWritePaths")).toEqual(["/srv/restic/headscale-vps/headscale"]);
    expect(value("LoadCredential")).toEqual(["restic-password:/etc/restic/headscale-password"]);
    expect(value("Environment")).toEqual(expect.arrayContaining(["RESTIC_PASSWORD_FILE=%d/restic-password"]));
  });
});
