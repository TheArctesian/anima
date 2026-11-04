import { describe, expect, test } from "bun:test";
import { readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./roles-setup.ts";

const BACKUP_FOLDER = join(ROOT, "deploy", "backup");
const unit = readFileSync(join(BACKUP_FOLDER, "anima-backup.unit"), "utf8");
const timer = readFileSync(join(BACKUP_FOLDER, "anima-backup.timer"), "utf8");

const lines = (text: string, key: string) =>
  text.split("\n").filter((line) => line.startsWith(`${key}=`)).map((line) => line.slice(key.length + 1));

describe("systemd units (DB-007, VPS-006)", () => {
  test("VPS-006: the unit runs the backup, then the off-site copy, from /opt/anima", () => {
    expect(lines(unit, "ExecStart")).toEqual([
      "/opt/anima/deploy/backup/backup.sh",
      "/opt/anima/deploy/backup/offsite.sh",
    ]);
    for (const script of ["backup.sh", "offsite.sh"]) {
      expect(statSync(join(BACKUP_FOLDER, script)).mode & 0o111).not.toBe(0);
    }
    expect(lines(unit, "Type")).toEqual(["oneshot"]);
    expect(lines(unit, "TimeoutStartSec")).toEqual(["2h"]);
  });

  test("DB-007: the sandbox keeps the backup folder writable and the rest read-only", () => {
    expect(lines(unit, "ProtectSystem")).toEqual(["strict"]);
    expect(lines(unit, "ReadWritePaths")).toEqual(["/var/backups/anima"]);
    expect(lines(unit, "NoNewPrivileges")).toEqual(["yes"]);
    expect(lines(unit, "PrivateTmp")).toEqual(["yes"]);
    expect(lines(unit, "UMask")).toEqual(["0077"]);
  });

  test("VPS-006: the timer fires daily, persists, and starts the service of the unit", () => {
    expect(lines(timer, "OnCalendar")).toEqual(["*-*-* 03:30:00 UTC"]);
    expect(lines(timer, "Persistent")).toEqual(["true"]);
    expect(lines(timer, "Unit")).toEqual(["anima-backup.service"]);
  });
});
