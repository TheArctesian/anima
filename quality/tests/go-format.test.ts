import { describe, expect, test } from "bun:test";
import { goFiles, unformatted } from "../src/gates/go-format.ts";

describe("go-fmt gate", () => {
  test("checks only Go files", () => {
    expect(goFiles(["link/go.mod", "link/devices/list.go", "crates/hub/src/main.rs"])).toEqual(["link/devices/list.go"]);
  });

  test("each file that gofmt would change is a violation", () => {
    expect(unformatted("link/devices/list.go\nlink/invites/code.go\n")).toEqual([
      { path: "link/devices/list.go", signal: "gofmt", detail: "run `gofmt -w` on this file" },
      { path: "link/invites/code.go", signal: "gofmt", detail: "run `gofmt -w` on this file" },
    ]);
  });

  test("no output is no violation", () => {
    expect(unformatted("")).toEqual([]);
  });
});
