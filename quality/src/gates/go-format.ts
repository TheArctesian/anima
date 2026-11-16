import type { Violation } from "../report.ts";

/** The Go files of a file list. */
export function goFiles(paths: string[]): string[] {
  return paths.filter((path) => path.endsWith(".go"));
}

/** `gofmt -l` prints one path for each file that it would change: each is a violation. */
export function unformatted(gofmtOutput: string): Violation[] {
  return gofmtOutput
    .split("\n")
    .filter((path) => path !== "")
    .map((path) => ({ path, signal: "gofmt", detail: "run `gofmt -w` on this file" }));
}
