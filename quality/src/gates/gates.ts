import { join } from "node:path";

/** An executable a gate needs on `PATH`, and how to install it. */
export interface Requirement {
  executable: string;
  install: string;
}

/** One local gate: a name, the argv that runs it from the repository root, and what it needs on `PATH`. */
export interface Gate {
  name: string;
  argv: string[];
  requires?: Requirement[];
}

/** What happened while running gates, in order. `code` is the exit code (1 when killed by a signal). */
export type GateEvent =
  | { kind: "start"; position: number; total: number; gate: Gate }
  | { kind: "pass"; position: number; total: number; gate: Gate; seconds: number }
  | { kind: "fail"; position: number; total: number; gate: Gate; code: number; message?: string }
  | { kind: "done"; passed: number; total: number; stoppedAt?: Gate };

export interface GateOutcome {
  exitCode: number;
  stoppedAt?: Gate;
}

/** Runs one gate to completion and returns its exit code. Throws when it cannot start. */
export type RunProcess = (gate: Gate) => number;
export type ReportGateEvent = (event: GateEvent) => void;

const NEXTEST: Requirement = { executable: "cargo-nextest", install: "cargo install cargo-nextest --locked" };

/**
 * The fourteen gates in the order they run (CQ-010, CQ-011). Gates 3 to 5 together run every test
 * `cargo test` runs (R-112): nextest the libtest tests, then the doctests, then the one non-libtest target.
 */
export function gateList(baseRef: string): Gate[] {
  return [
    { name: "fmt", argv: ["cargo", "fmt", "--check"] },
    { name: "clippy", argv: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"] },
    // Rust tests run against a throwaway PostgreSQL unless DATABASE_URL is set (R-009).
    {
      name: "rust-test",
      argv: ["bun", "quality/bin/with-postgres.ts", "--migrated-template", "cargo", "nextest", "run", "--workspace"],
      requires: [NEXTEST],
    },
    { name: "doctest", argv: ["cargo", "test", "--workspace", "--doc"] },
    // `harness = false`, so not listable by nextest (.config/nextest.toml); `--workspace` keeps gate 3's build.
    { name: "pinning", argv: ["cargo", "test", "--workspace", "--test", "pinning"] },
    { name: "install", argv: ["bun", "install", "--frozen-lockfile"] },
    { name: "lint", argv: ["bun", "run", "lint"] },
    { name: "check", argv: ["bun", "run", "check"] },
    { name: "test", argv: ["bun", "run", "test"] },
    { name: "build", argv: ["bun", "run", "--cwd", "apps/client", "build"] },
    { name: "limits", argv: ["bun", "run", "quality:limits"] },
    { name: "names", argv: ["bun", "run", "quality:names"] },
    { name: "imports", argv: ["bun", "run", "quality:imports"] },
    { name: "baseline", argv: ["bun", "run", "quality:baseline", "--", "--base", baseRef] },
  ];
}

/** Run gates in order and stop at the first failure. A runner that throws is a failure, never a skip (CQ-024). */
export function runGates(
  gates: Gate[],
  run: RunProcess,
  report: ReportGateEvent,
  now: () => number = Date.now,
): GateOutcome {
  const total = gates.length;
  for (const [index, gate] of gates.entries()) {
    const position = index + 1;
    report({ kind: "start", position, total, gate });
    const began = now();
    let code: number;
    try {
      code = run(gate);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      report({ kind: "fail", position, total, gate, code: 1, message });
      report({ kind: "done", passed: index, total, stoppedAt: gate });
      return { exitCode: 1, stoppedAt: gate };
    }
    if (code !== 0) {
      report({ kind: "fail", position, total, gate, code });
      report({ kind: "done", passed: index, total, stoppedAt: gate });
      return { exitCode: code, stoppedAt: gate };
    }
    report({ kind: "pass", position, total, gate, seconds: (now() - began) / 1000 });
  }
  report({ kind: "done", passed: total, total });
  return { exitCode: 0 };
}

/**
 * Why `gate` cannot run: the first required executable missing from every `PATH` folder.
 * A missing tool fails its gate, never skips it (CQ-024).
 */
export function missingRequirement(gate: Gate, pathEnv: string, exists: (path: string) => boolean): string | undefined {
  const dirs = pathEnv.split(":").filter((dir) => dir !== "");
  const missing = (gate.requires ?? []).find((need) => !dirs.some((dir) => exists(join(dir, need.executable))));
  if (missing === undefined) return undefined;
  return `${missing.executable} is not on PATH; install it with \`${missing.install}\` and put its folder (~/.cargo/bin) on PATH`;
}

export function formatGateEvent(event: GateEvent): string {
  switch (event.kind) {
    case "start":
      return `[${event.position}/${event.total}] ${event.gate.name}: ${event.gate.argv.join(" ")}`;
    case "pass":
      return `[${event.position}/${event.total}] ${event.gate.name}: pass (${event.seconds.toFixed(1)}s)`;
    case "fail":
      return `[${event.position}/${event.total}] ${event.gate.name}: FAIL (exit ${event.code})${event.message ? ` ${event.message}` : ""}`;
    case "done":
      return event.stoppedAt
        ? `gates: stopped at ${event.stoppedAt.name}`
        : `gates: ${event.passed}/${event.total} pass`;
  }
}
