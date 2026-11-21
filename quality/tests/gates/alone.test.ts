import { describe, expect, test } from "bun:test";
import { checkAlone } from "../../src/gates/alone.ts";

describe("each crate alone (gates.ts --alone)", () => {
  test("each crate gets its own cargo check, and one that does not build alone fails the run and is named", () => {
    const ran: string[] = [];
    const lines: string[] = [];
    const code = checkAlone(["anima-hub", "anima-cli", "anima-app"], (argv) => {
      ran.push(argv.join(" "));
      return argv.includes("anima-cli") ? 101 : 0;
    }, (line) => lines.push(line));
    expect(ran).toEqual(["cargo check -p anima-hub --locked", "cargo check -p anima-cli --locked", "cargo check -p anima-app --locked"]);
    expect(code).toBe(1);
    expect(lines.at(-1)).toContain("do not build alone: anima-cli");
  });

  test("when each crate builds alone the run passes", () => {
    expect(checkAlone(["anima-hub"], () => 0, () => undefined)).toBe(0);
  });
});
