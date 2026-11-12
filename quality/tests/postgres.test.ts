import { describe, expect, test } from "bun:test";
import { binaryDirs, findTools, serverOptions, socketDatabaseUrl } from "../src/gates/postgres.ts";

describe("throwaway PostgreSQL (R-009)", () => {
  test("prefers the versioned Homebrew and Debian folders, then PATH", () => {
    const dirs = binaryDirs("/usr/bin:/bin", 17);
    expect(dirs[0]).toBe("/opt/homebrew/opt/postgresql@17/bin");
    expect(dirs).toContain("/usr/lib/postgresql/17/bin");
    expect(dirs.slice(-2)).toEqual(["/usr/bin", "/bin"]);
  });

  test("finds the first folder that has every tool", () => {
    const present = new Set(["/b/initdb", "/b/pg_ctl", "/a/initdb"]);
    expect(findTools(["/a", "/b"], ["initdb", "pg_ctl"], (path) => present.has(path))).toBe("/b");
    expect(findTools(["/a"], ["initdb", "pg_ctl"], (path) => present.has(path))).toBeUndefined();
  });

  test("builds a socket-only URL that sqlx understands", () => {
    expect(socketDatabaseUrl("/tmp/anima-pg-x", 5432, "op")).toBe(
      "postgres://op@localhost:5432/postgres?host=%2Ftmp%2Fanima-pg-x",
    );
  });

  test("starts the server with no TCP listener", () => {
    expect(serverOptions("/tmp/s", 5432)).toContain("listen_addresses=''");
  });
});
