import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { delimiter, join } from "node:path";
import { compileCache } from "../src/gates/cache.ts";

const onPath = (...files: string[]) => (path: string) => files.includes(path);

describe("compileCache", () => {
  test("uses sccache from PATH and turns incremental builds off", () => {
    const path = ["/usr/bin", "/opt/homebrew/bin"].join(delimiter);
    const wrapper = join("/opt/homebrew/bin", process.platform === "win32" ? "sccache.exe" : "sccache");
    const { env, note } = compileCache({ PATH: path, HOME: "/h" }, onPath(wrapper));
    expect(env).toEqual({ PATH: path, HOME: "/h", RUSTC_WRAPPER: wrapper, CARGO_INCREMENTAL: "0" });
    expect(note).toBe(`rust cache: sccache (${wrapper})`);
  });

  test("runs without a cache, and says how to get one, when sccache is not on PATH", () => {
    const given = { PATH: "/usr/bin" };
    const { env, note } = compileCache(given, onPath());
    expect(env).toEqual(process.platform === "win32" ? { ...given, RUSTC_WRAPPER: "" } : given);
    expect(note).toContain("brew install sccache");
  });

  test("keeps a RUSTC_WRAPPER the caller set, even an empty one", () => {
    for (const wrapper of ["/x/other", ""]) {
      const given = { PATH: "/opt/homebrew/bin", RUSTC_WRAPPER: wrapper };
      expect(compileCache(given, onPath("/opt/homebrew/bin/sccache")).env).toBe(given);
    }
  });
});

describe("the compiler wrapper of .cargo/config.toml (quality/bin/rustc-cache.sh)", () => {
  const wrapper = join(import.meta.dir, "..", "bin", "rustc-cache.sh");

  // Windows uses a native environment override, never this Unix wrapper.
  test.skipIf(process.platform === "win32")("runs the compiler directly when sccache is not on PATH, and keeps a variable with a hyphen in its name", () => {
    // cargo names the program of a test `CARGO_BIN_EXE_<name>`, and a name can have a hyphen.
    const run = Bun.spawnSync([wrapper, "/usr/bin/env"], {
      env: { PATH: "/usr/bin:/bin", "CARGO_BIN_EXE_anima-local-hub": "/target/debug/anima-local-hub" },
    });
    expect(run.exitCode).toBe(0);
    expect(run.stdout.toString()).toContain("CARGO_BIN_EXE_anima-local-hub=/target/debug/anima-local-hub");
  });

  test("is not a script of sh, whose dash form drops such a variable", () => {
    expect(readFileSync(wrapper, "utf8").split("\n")[0]).toBe("#!/usr/bin/env bash");
  });
});

