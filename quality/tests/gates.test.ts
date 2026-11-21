import { describe, expect, test } from "bun:test";
import { delimiter, join } from "node:path";
import { formatGateEvent, gateList, keptGoing, missingRequirement, runGates, type Gate, type GateEvent } from "../src/gates/gates.ts";

const NAMES = [
  "fmt", "clippy", "rust-test", "doctest", "pinning",
  "go-fmt", "go-vet", "go-staticcheck", "go-revive", "go-test", "go-build", "install", "lint", "check", "test", "build", "limits", "names", "imports", "documents", "baseline",
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
  test("CQ-010 lists the twenty-one gates in the documented order", () => {
    expect(gateList("phase-1").map((gate) => gate.name)).toEqual(NAMES);
  });

  test("R-009 R-112 runs the Rust tests with nextest against a throwaway PostgreSQL", () => {
    const rustTest = gateList("phase-1").find((gate) => gate.name === "rust-test");
    expect(rustTest?.argv).toEqual([
      "bun", "quality/bin/with-postgres.ts", "cargo", "nextest", "run", "--workspace",
    ]);
  });

  test("R-112 still runs the doctests and the non-libtest target that nextest does not", () => {
    const argvs = gateList("phase-1").map((gate) => gate.argv.join(" "));
    expect(argvs).toContain("cargo test --workspace --doc");
    expect(argvs).toContain("cargo test --workspace --test pinning");
  });

  test("LNK runs each Go command inside the link module, with its lint tools from go.mod", () => {
    const argvs = gateList("phase-1").map((gate) => gate.argv.join(" "));
    expect(argvs).toContain("go -C link vet ./...");
    expect(argvs).toContain("go -C link tool staticcheck ./...");
    expect(argvs).toContain("go -C link tool revive -config revive.toml -set_exit_status ./...");
    expect(argvs).toContain("go -C link test -race ./...");
    expect(argvs).toContain(`go -C link build -o ${process.platform === "win32" ? "NUL" : "/dev/null"} ./...`);
  });

  test("CQ-011 passes the base ref to the baseline gate", () => {
    const baseline = gateList("abc").at(-1);
    expect(baseline?.argv).toEqual(["bun", "run", "quality:baseline", "--", "--base", "abc"]);
  });
});

describe("runGates", () => {
  test("CQ-010 runs all twenty-one gates in order and succeeds", () => {
    const { ran, events, run, report } = recorder();
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(outcome).toEqual({ exitCode: 0 });
    expect(ran).toEqual(gates.map((gate) => gate.argv));
    expect(events.at(-1)).toEqual({ kind: "done", passed: 21, total: 21 });
  });

  test("CQ-010 stops at the first failing gate and returns its exit code", () => {
    const { ran, events, run, report } = recorder({ 3: 101 });
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report);
    expect(ran).toHaveLength(3);
    expect(outcome).toEqual({ exitCode: 101, stoppedAt: gates[2] });
    expect(events.at(-1)).toEqual({ kind: "done", passed: 2, total: 21, stoppedAt: gates[2] });
  });

  test("CQ-024 treats a runner that throws as a failure, not a skip", () => {
    const { ran, events, run, report } = recorder({ 1: new Error("cargo: not found") });
    const outcome = runGates(gateList("phase-1"), run, report);
    expect(ran).toHaveLength(1);
    expect(outcome.exitCode).toBe(1);
    expect(events).toContainEqual(expect.objectContaining({ kind: "fail", code: 1, message: "cargo: not found" }));
  });
});

describe("runGates with keepGoing (a proof run)", () => {
  test("runs each gate after failures, fails with the first exit code and names each failed gate", () => {
    const { ran, events, run, report } = recorder({ 2: 101, 3: new Error("cargo-nextest is not on PATH"), 13: 1 });
    const gates = gateList("phase-1");
    const outcome = runGates(gates, run, report, { keepGoing: true });
    expect(ran).toHaveLength(21);
    expect(outcome).toEqual({ exitCode: 101, failed: [gates[1], gates[2], gates[12]] });
    const done = events.at(-1);
    expect(done).toEqual({ kind: "done", passed: 18, total: 21, failed: [gates[1], gates[2], gates[12]] });
    if (done !== undefined) expect(formatGateEvent(done)).toBe("gates: 18/21 pass; failed: clippy, rust-test, lint");
  });

  test("passes when each gate passed", () => {
    const { run, report } = recorder();
    expect(runGates(gateList("phase-1"), run, report, { keepGoing: true })).toEqual({ exitCode: 0 });
  });

  test("goes on inside cargo too: each crate's lints and each failing test", () => {
    const argv = (name: string) => keptGoing(gateList("phase-1")).find((gate) => gate.name === name)?.argv.join(" ");
    expect(argv("clippy")).toBe("cargo clippy --all-targets --keep-going -- -D warnings");
    expect(argv("rust-test")).toBe("bun quality/bin/with-postgres.ts cargo nextest run --workspace --no-fail-fast");
    expect(argv("doctest")).toBe("cargo test --workspace --doc --no-fail-fast");
    expect(argv("pinning")).toBe("cargo test --workspace --test pinning --no-fail-fast");
    expect(argv("lint")).toBe("bun run lint");
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
    const present = (path: string) => path === join("/home/op/.cargo/bin", process.platform === "win32" ? "cargo-nextest.exe" : "cargo-nextest");
    expect(missingRequirement(rustTest, ["/usr/bin", "/home/op/.cargo/bin"].join(delimiter), present)).toBeUndefined();
  });

  test("CQ-024 fails a Go gate with the install command when go is not on PATH", () => {
    const goVet = gateList("phase-1").find((gate) => gate.name === "go-vet");
    if (goVet === undefined) throw new Error("no go-vet gate");
    const message = missingRequirement(goVet, "/usr/bin:/bin", () => false);
    expect(message).toContain("go is not on PATH");
    expect(message).toContain("brew install go");
    expect(message).not.toContain(".cargo");
  });

  test("CQ-024 fails the format gate when gofmt is not on PATH", () => {
    const goFmt = gateList("phase-1").find((gate) => gate.name === "go-fmt");
    if (goFmt === undefined) throw new Error("no go-fmt gate");
    expect(missingRequirement(goFmt, "/usr/bin", () => false)).toContain("gofmt is not on PATH");
  });

  test("needs nothing for a gate without requirements", () => {
    expect(missingRequirement({ name: "fmt", group: "rust", argv: ["cargo", "fmt"] }, "", () => false)).toBeUndefined();
  });
});

