import { describe, expect, test } from "bun:test";
import { compileCache, rustcEnv } from "../src/gates/compile-cache.ts";

const WRAPPER = "/repo/quality/bin/compile-cache.ts";
const onlyIn = (dir: string) => (path: string) => path === `${dir}/sccache`;

describe("compileCache (R-112, R-101)", () => {
  test("wraps rustc with the first sccache on PATH and keeps the rest of the environment", () => {
    const env = { PATH: "/usr/bin:/home/op/.cargo/bin", CARGO_TARGET_DIR: "/w/target", CARGO_INCREMENTAL: "0" };
    const cache = compileCache(env, onlyIn("/home/op/.cargo/bin"), WRAPPER);
    expect(cache.env).toEqual({ ...env, RUSTC_WRAPPER: WRAPPER, ANIMA_SCCACHE: "/home/op/.cargo/bin/sccache" });
    expect(cache.line).toContain("on");
  });

  test("compiles with plain rustc and says so when sccache is missing", () => {
    const env = { PATH: "/usr/bin" };
    const cache = compileCache(env, () => false, WRAPPER);
    expect(cache.env).toEqual(env);
    expect(cache.line).toContain("off");
  });

  test("leaves a RUSTC_WRAPPER the caller set, even an empty one", () => {
    for (const wrapper of ["/opt/other-wrapper", ""]) {
      const env = { PATH: "/home/op/.cargo/bin", RUSTC_WRAPPER: wrapper };
      expect(compileCache(env, onlyIn("/home/op/.cargo/bin"), WRAPPER).env).toEqual(env);
    }
  });
});

describe("rustcEnv (R-101)", () => {
  test("hides the checkout's target folder from sccache's key and keeps every other variable", () => {
    const env = {
      CARGO_TARGET_DIR: "/w/target",
      CARGO_BUILD_TARGET_DIR: "/w/target",
      CARGO_INCREMENTAL: "0",
      "CARGO_BIN_EXE_anima-hub": "/w/target/debug/anima-hub",
      PATH: "/usr/bin",
    };
    expect(rustcEnv(env)).toEqual({
      CARGO_INCREMENTAL: "0",
      "CARGO_BIN_EXE_anima-hub": "/w/target/debug/anima-hub",
      PATH: "/usr/bin",
    });
  });
});
