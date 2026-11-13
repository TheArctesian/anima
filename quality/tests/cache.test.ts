import { describe, expect, test } from "bun:test";
import { compileCache } from "../src/gates/cache.ts";

const onPath = (...files: string[]) => (path: string) => files.includes(path);

describe("compileCache", () => {
  test("uses sccache from PATH and turns incremental builds off", () => {
    const { env, note } = compileCache({ PATH: "/usr/bin:/opt/homebrew/bin", HOME: "/h" }, onPath("/opt/homebrew/bin/sccache"));
    expect(env).toEqual({ PATH: "/usr/bin:/opt/homebrew/bin", HOME: "/h", RUSTC_WRAPPER: "/opt/homebrew/bin/sccache", CARGO_INCREMENTAL: "0" });
    expect(note).toBe("rust cache: sccache (/opt/homebrew/bin/sccache)");
  });

  test("runs without a cache, and says how to get one, when sccache is not on PATH", () => {
    const given = { PATH: "/usr/bin" };
    const { env, note } = compileCache(given, onPath());
    expect(env).toBe(given);
    expect(note).toContain("brew install sccache");
  });

  test("keeps a RUSTC_WRAPPER the caller set, even an empty one", () => {
    for (const wrapper of ["/x/other", ""]) {
      const given = { PATH: "/opt/homebrew/bin", RUSTC_WRAPPER: wrapper };
      expect(compileCache(given, onPath("/opt/homebrew/bin/sccache")).env).toBe(given);
    }
  });
});
