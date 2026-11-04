import { baselineValue, type Baseline, type Signal } from "./baseline.ts";
import { countCodeLines } from "./codelines.ts";
import type { Violation } from "./report.ts";

export const MEASURED_EXTENSIONS = [".rs", ".ts", ".js", ".svelte", ".css"];
export const FILE_LINES_LIMIT = 300;
export const FOLDER_FILES_LIMIT = 12;
const FOLDER_ROOTS = ["crates/", "apps/", "quality/", "design/"];

export interface SourceFile {
  path: string;
  text: string;
}

export function extensionOf(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot === -1 ? "" : name.slice(dot);
}

export function isMeasured(path: string): boolean {
  return MEASURED_EXTENSIONS.includes(extensionOf(path));
}

function overLimit(
  baseline: Baseline,
  path: string,
  signal: Signal,
  value: number,
  limit: number,
): Violation[] {
  if (value <= limit) return [];
  const allowed = baselineValue(baseline, path, signal);
  if (allowed !== undefined && allowed >= value) return [];
  return [{ path, signal, detail: `${value} > ${allowed ?? limit}` }];
}

function fileLines(files: SourceFile[], baseline: Baseline): Violation[] {
  return files
    .filter((file) => isMeasured(file.path))
    .flatMap((file) =>
      overLimit(
        baseline,
        file.path,
        "file-lines",
        countCodeLines(file.text, extensionOf(file.path)),
        FILE_LINES_LIMIT,
      ),
    );
}

function folderFiles(files: SourceFile[], baseline: Baseline): Violation[] {
  const counts = new Map<string, number>();
  for (const { path } of files) {
    if (!isMeasured(path) || !FOLDER_ROOTS.some((root) => path.startsWith(root))) continue;
    const folder = path.slice(0, path.lastIndexOf("/"));
    counts.set(folder, (counts.get(folder) ?? 0) + 1);
  }
  return [...counts].flatMap(([folder, count]) =>
    overLimit(baseline, folder, "folder-files", count, FOLDER_FILES_LIMIT),
  );
}

/** CQ-010, REPO-005: code lines per file and measured files per folder. */
export function checkLimits(files: SourceFile[], baseline: Baseline): Violation[] {
  return [...fileLines(files, baseline), ...folderFiles(files, baseline)];
}
