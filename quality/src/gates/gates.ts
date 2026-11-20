import { delimiter, join } from "node:path";

/** An executable a gate needs on `PATH`, how to install it, and the folder that install puts it in. */
export interface Requirement {
  executable: string;
  install: string;
  folder: string;
}

/** The part of the repository a gate checks. A job on GitHub runs one or more groups (`--group`). */
export type Group = "rust" | "go" | "bun" | "quality";
export const GROUPS: readonly Group[] = ["rust", "go", "bun", "quality"];

/** One local gate: a name, its group, the argv that runs it from the repository root, and what it needs on `PATH`. */
export interface Gate {
  name: string;
  group: Group;
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

const NEXTEST: Requirement = {
  executable: "cargo-nextest",
  install: "cargo install cargo-nextest --locked",
  folder: "~/.cargo/bin",
};
const GO: Requirement = { executable: "go", install: "brew install go", folder: "/opt/homebrew/bin, or /usr/local/go/bin" };
const GOFMT: Requirement = { ...GO, executable: "gofmt" };

/** The Go module of the link (LNK). `go -C` runs each command in it. */
const GO_MODULE = "link";

function goGate(name: string, ...args: string[]): Gate {
  return { name, group: "go", argv: ["go", "-C", GO_MODULE, ...args], requires: [GO] };
}

/**
 * The twenty gates in the order they run (CQ-010, CQ-011). Gates 3 to 5 together run every test
 * `cargo test` runs (R-112): nextest the libtest tests, then the doctests, then the one non-libtest target.
 * Gates 6 to 11 are the Go gates of the `link/` module; its lint tools are pinned in `link/go.mod`.
 */
export function gateList(baseRef: string): Gate[] {
  return [
    { name: "fmt", group: "rust", argv: ["cargo", "fmt", "--check"] },
    { name: "clippy", group: "rust", argv: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"] },
    // Rust tests run against a throwaway PostgreSQL unless DATABASE_URL is set (R-009).
    {
      name: "rust-test",
      group: "rust",
      argv: ["bun", "quality/bin/with-postgres.ts", "cargo", "nextest", "run", "--workspace"],
      requires: [NEXTEST],
    },
    { name: "doctest", group: "rust", argv: ["cargo", "test", "--workspace", "--doc"] },
    // `harness = false`, so not listable by nextest (.config/nextest.toml); `--workspace` keeps gate 3's build.
    { name: "pinning", group: "rust", argv: ["cargo", "test", "--workspace", "--test", "pinning"] },
    { name: "go-fmt", group: "go", argv: ["bun", "quality/bin/go-format.ts"], requires: [GOFMT] },
    goGate("go-vet", "vet", "./..."),
    goGate("go-staticcheck", "tool", "staticcheck", "./..."),
    goGate("go-revive", "tool", "revive", "-config", "revive.toml", "-set_exit_status", "./..."),
    goGate("go-test", "test", "-race", "./..."),
    // `-o /dev/null`: with one main package, a plain `go build` writes the program into `link/`.
    goGate("go-build", "build", "-o", process.platform === "win32" ? "NUL" : "/dev/null", "./..."),
    { name: "install", group: "bun", argv: ["bun", "install", "--frozen-lockfile"] },
    { name: "lint", group: "bun", argv: ["bun", "run", "lint"] },
    { name: "check", group: "bun", argv: ["bun", "run", "check"] },
    { name: "test", group: "bun", argv: ["bun", "run", "test"] },
    { name: "build", group: "bun", argv: ["bun", "run", "--cwd", "apps/client", "build"] },
    { name: "limits", group: "quality", argv: ["bun", "run", "quality:limits"] },
    { name: "names", group: "quality", argv: ["bun", "run", "quality:names"] },
    { name: "imports", group: "quality", argv: ["bun", "run", "quality:imports"] },
    { name: "documents", group: "quality", argv: ["bun", "run", "quality:documents"] },
    { name: "baseline", group: "quality", argv: ["bun", "run", "quality:baseline", "--", "--base", baseRef] },
  ];
}

/** The gates of `groups`, in the order of the list. A job on GitHub runs a part of the list this way. */
export function ofGroups(gates: Gate[], groups: readonly Group[]): Gate[] {
  return gates.filter((gate) => groups.includes(gate.group));
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
  const dirs = pathEnv.split(delimiter).filter((dir) => dir !== "");
  const missing = (gate.requires ?? []).find((need) => !dirs.some((dir) => exists(join(dir, process.platform === "win32" ? `${need.executable}.exe` : need.executable))));
  if (missing === undefined) return undefined;
  return `${missing.executable} is not on PATH; install it with \`${missing.install}\` and put its folder (${missing.folder}) on PATH`;
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
