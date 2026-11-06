import { describe, expect, test } from "bun:test";
import { gateList, runGates, type GateEvent } from "../src/gates/gates.ts";

const NAMES = [
  "fmt", "clippy", "cargo-test", "install", "lint", "check", "test", "build", "limits", "names", "imports", "baseline",
];

function recorder(codes: Record<number, number | Error> = {}) {
  const ran: string[][] = [];
  const events: GateEvent[] = [];
  const run = (argv: string[]): number => {
    ran.push(argv);
    const planned = codes[ran.length];
    if (planned instanceof Error) throw planned;
    return planned ?? 0;
  };
  return { ran, events, run, report: (event: GateEvent) => events.push(event) };
}

describe("gateList", () => {
  test("CQ-010 lists the twelve gates in the documented order", () => {
    expect(gateList("phase-1").map((gate) => gate.name)).toEqual(NAMES);
  });

  test("R-009 runs the Rust tests with a throwaway PostgreSQL", () => {
    const cargoTest = gateList("phase-1").find((gate) => gate.name === "cargo-test");
    expect(cargoTest?.argv).toEqual(["bun", "quality/bin/with-postgres.ts", "cargo", "test"]);
  });

  test("CQ-011 passes the base ref to the baseline gate", () => {
    const baseline = gateList("abc").at(-1);
    expect(baseline?.argv).toEqual(["bun", "run", "quality:baseline", "--", "--base", "abc"]);
  });
});

describe("runGates", () => {
  test("CQ-010 runs all twelve gates in order and succeeds", () => {
    const { ran, events, run, report } = recorder();
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(outcome).toEqual({ exitCode: 0 });
    expect(ran).toEqual(gates.map((gate) => gate.argv));
    expect(events.at(-1)).toEqual({ kind: "done", passed: 12, total: 12 });
  });

  test("CQ-010 stops at the first failing gate and returns its exit code", () => {
    const { ran, events, run, report } = recorder({ 3: 101 });
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(ran).toHaveLength(3);
    expect(outcome).toEqual({ exitCode: 101, stoppedAt: gates[2] });
    expect(events.at(-1)).toEqual({ kind: "done", passed: 2, total: 12, stoppedAt: gates[2] });
  });

  test("CQ-024 treats a runner that throws as a failure, not a skip", () => {
    const { ran, events, run, report } = recorder({ 1: new Error("cargo: not found") });
    const outcome = runGates(gateList("phase-1"), run, report);
    expect(ran).toHaveLength(1);
    expect(outcome.exitCode).toBe(1);
    expect(events).toContainEqual(expect.objectContaining({ kind: "fail", code: 1, message: "cargo: not found" }));
  });
});
