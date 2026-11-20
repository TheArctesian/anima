import { expect, test } from "bun:test";
import { join } from "node:path";

test("native bundle commands disable the Unix Rust wrapper and preserve arguments", () => {
  const script = join(import.meta.dir, "..", "bin", "windows-command.ts");
  const words = ["a space", "apostrophe's", "雪", "-flag=value"];
  const result = Bun.spawnSync([
    process.execPath, script, process.execPath, "-e",
    "console.log(JSON.stringify([process.env.RUSTC_WRAPPER, ...process.argv.slice(1)]))", ...words,
  ], { env: { ...process.env, RUSTC_WRAPPER: "must-not-run" } });
  expect(result.exitCode).toBe(0);
  expect(JSON.parse(result.stdout.toString()) as unknown).toEqual(["", ...words]);
});
