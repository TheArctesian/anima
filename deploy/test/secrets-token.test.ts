import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { makeSecrets, newSecretsPath, read, removeScratchFolders, scratchFolder, shimFolder, startMakeSecrets } from "./make-secrets.ts";

afterEach(removeScratchFolders);

const TOKEN_ARGS = ["--cloudflare-token-stdin"];

function tokenFile(folder: string): string {
  return join(folder, "cloudflare_dns_token");
}

describe("make-secrets Cloudflare token from stdin (DNS-003)", () => {
  test("DNS-003: it stores the first line with mode 0400 and prints nothing of it", () => {
    const folder = newSecretsPath();
    const result = makeSecrets(folder, { args: TOKEN_ARGS, stdin: "cf-secret-token\nextra\n" });
    expect(result.exitCode).toBe(0);
    expect(read(folder, "cloudflare_dns_token")).toBe("cf-secret-token");
    expect(result.output).not.toContain("cf-secret-token");
    expect(Bun.spawnSync(["ls", "-l", tokenFile(folder)]).stdout.toString()).toMatch(/^-r--------/);
  });

  test("DNS-003: one trailing carriage return is removed", () => {
    const folder = newSecretsPath();
    makeSecrets(folder, { args: TOKEN_ARGS, stdin: "abc_-9\r\n" });
    expect(read(folder, "cloudflare_dns_token")).toBe("abc_-9");
  });

  test.each([["empty", "\n"], ["with a space", "ab cd\n"], ["with a colon", "tok:en\n"], ["with two returns", "abc\r\r\n"]])(
    "DNS-003: a token that is %s gives exit 1, no file and no echo of the value",
    (_name, line) => {
      const folder = newSecretsPath();
      const result = makeSecrets(folder, { args: TOKEN_ARGS, stdin: line });
      expect(result.exitCode).toBe(1);
      expect(existsSync(tokenFile(folder))).toBe(false);
      if (line.trim() !== "") expect(result.output).not.toContain(line.trim());
    },
  );
});

describe("make-secrets token prompt (DNS-003)", () => {
  function sttyShim(): { shims: string; log: string } {
    const log = join(scratchFolder(), "stty.log");
    return { shims: shimFolder({ stty: `echo "$*" >>"${log}"` }), log };
  }

  test("DNS-003: the prompt turns echo off before it reads and on afterwards", () => {
    const folder = newSecretsPath();
    const { shims, log } = sttyShim();
    const result = makeSecrets(folder, { shimDirectories: [shims], assumeTerminal: true, stdin: "prompted-token\n" });
    expect(result.exitCode).toBe(0);
    expect(read(folder, "cloudflare_dns_token")).toBe("prompted-token");
    expect(result.output).not.toContain("prompted-token");
    const calls = readFileSync(log, "utf8").trim().split("\n");
    expect(calls.slice(0, 2)).toEqual(["-echo", "echo"]);
  });

  test("DNS-003: with --cloudflare-token-stdin on a terminal it still reads with echo off", () => {
    const folder = newSecretsPath();
    const { shims, log } = sttyShim();
    makeSecrets(folder, { shimDirectories: [shims], assumeTerminal: true, args: TOKEN_ARGS, stdin: "tok\n" });
    expect(readFileSync(log, "utf8").split("\n")[0]).toBe("-echo");
  });

  test("DNS-003: an interrupt during the read restores echo, exits 130 and writes no token", async () => {
    const folder = newSecretsPath();
    const { shims, log } = sttyShim();
    const child = startMakeSecrets(folder, { shimDirectories: [shims], assumeTerminal: true });
    for (let waited = 0; !existsSync(log) && waited < 100; waited++) await Bun.sleep(50);
    expect(readFileSync(log, "utf8")).toContain("-echo");
    child.kill("SIGINT");
    expect(await child.exited).toBe(130);
    const [first, ...rest] = readFileSync(log, "utf8").trim().split("\n");
    expect(first).toBe("-echo");
    expect(rest.length).toBeGreaterThan(0);
    expect(rest.every((call) => call === "echo")).toBe(true);
    expect(existsSync(tokenFile(folder))).toBe(false);
  });
});
