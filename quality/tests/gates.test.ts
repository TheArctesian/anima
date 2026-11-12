import { describe, expect, test } from "bun:test";
import { gateList, missingRequirement, runGates, type Gate, type GateEvent } from "../src/gates/gates.ts";

const NAMES = [
  "fmt", "clippy", "rust-test", "doctest", "pinning", "install", "lint", "check", "test", "build", "limits", "names", "imports", "baseline",
];

function recorder(codes: Record<number, number | Error> = {}) {
  const ran: string[][] = [];
  const events: GateEvent[] = [];
  const run = (gate: Gate): number => {
    ran.push(gate.argv);
    const planned = codes[ran.length];
    if (planned instanceof Error) throw planned;
    return planned ?? 0;
  };
  return { ran, events, run, report: (event: GateEvent) => events.push(event) };
}

describe("gateList", () => {
  test("CQ-010 lists the fourteen gates in the documented order", () => {
    expect(gateList("phase-1").map((gate) => gate.name)).toEqual(NAMES);
  });

  test("R-009 R-112 runs the Rust tests with nextest against a throwaway PostgreSQL", () => {
    const rustTest = gateList("phase-1").find((gate) => gate.name === "rust-test");
    expect(rustTest?.argv).toEqual(["bun", "quality/bin/with-postgres.ts", "cargo", "nextest", "run", "--workspace"]);
  });

  test("R-112 still runs the doctests and the non-libtest target that nextest does not", () => {
    const argvs = gateList("phase-1").map((gate) => gate.argv.join(" "));
    expect(argvs).toContain("cargo test --workspace --doc");
    expect(argvs).toContain("cargo test --workspace --test pinning");
  });

  test("CQ-011 passes the base ref to the baseline gate", () => {
    const baseline = gateList("abc").at(-1);
    expect(baseline?.argv).toEqual(["bun", "run", "quality:baseline", "--", "--base", "abc"]);
  });
});

describe("runGates", () => {
  test("CQ-010 runs all fourteen gates in order and succeeds", () => {
    const { ran, events, run, report } = recorder();
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(outcome).toEqual({ exitCode: 0 });
    expect(ran).toEqual(gates.map((gate) => gate.argv));
    expect(events.at(-1)).toEqual({ kind: "done", passed: 14, total: 14 });
  });

  test("CQ-010 stops at the first failing gate and returns its exit code", () => {
    const { ran, events, run, report } = recorder({ 3: 101 });
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(ran).toHaveLength(3);
    expect(outcome).toEqual({ exitCode: 101, stoppedAt: gates[2] });
    expect(events.at(-1)).toEqual({ kind: "done", passed: 2, total: 14, stoppedAt: gates[2] });
  });

  test("CQ-024 treats a runner that throws as a failure, not a skip", () => {
    const { ran, events, run, report } = recorder({ 1: new Error("cargo: not found") });
    const outcome = runGates(gateList("phase-1"), run, report);
    expect(ran).toHaveLength(1);
    expect(outcome.exitCode).toBe(1);
    expect(events).toContainEqual(expect.objectContaining({ kind: "fail", code: 1, message: "cargo: not found" }));
  });
});

describe("missingRequirement", () => {
  const rustTest = gateList("phase-1").find((gate) => gate.name === "rust-test");
  if (rustTest === undefined) throw new Error("no rust-test gate");

  test("CQ-024 fails the nextest gate with the install command when cargo-nextest is not on PATH", () => {
    const message = missingRequirement(rustTest, "/usr/bin:/bin", () => false);
    expect(message).toContain("cargo-nextest");
    expect(message).toContain("cargo install cargo-nextest --locked");
  });

  test("lets the gate run when cargo-nextest is in a PATH folder", () => {
    const present = (path: string) => path === "/home/op/.cargo/bin/cargo-nextest";
    expect(missingRequirement(rustTest, "/usr/bin:/home/op/.cargo/bin", present)).toBeUndefined();
  });

  test("needs nothing for a gate without requirements", () => {
    expect(missingRequirement({ name: "fmt", argv: ["cargo", "fmt"] }, "", () => false)).toBeUndefined();
  });
});
