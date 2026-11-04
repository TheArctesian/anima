import { baselineValue, type Baseline, type Signal } from "./baseline.ts";
import { countCodeLines } from "./lexing.ts";
import type { Violation } from "./report.ts";
import { extensionOf, type SourceFile } from "./source.ts";

export const MEASURED_EXTENSIONS = [".rs", ".ts", ".js", ".svelte", ".css"];
export const FILE_LINES_LIMIT = 300;
export const FOLDER_FILES_LIMIT = 12;
const FOLDER_ROOTS = ["crates/", "apps/", "quality/", "design/"];

export function isMeasured(path: string): boolean {
  return MEASURED_EXTENSIONS.includes(extensionOf(path));
}

interface Measure {
  path: string;
  signal: Signal;
  value: number;
  limit: number;
}

function overLimit(baseline: Baseline, { path, signal, value, limit }: Measure): Violation[] {
  if (value <= limit) return [];
  const allowed = baselineValue(baseline, path, signal);
  if (allowed !== undefined && allowed >= value) return [];
  return [{ path, signal, detail: `${value} > ${allowed ?? limit}` }];
}

function fileLines(files: SourceFile[], baseline: Baseline): Violation[] {
  return files
    .filter((file) => isMeasured(file.path))
    .flatMap((file) =>
      overLimit(baseline, {
        path: file.path,
        signal: "file-lines",
        value: countCodeLines(file.text, extensionOf(file.path)),
        limit: FILE_LINES_LIMIT,
      }),
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
    overLimit(baseline, { path: folder, signal: "folder-files", value: count, limit: FOLDER_FILES_LIMIT }),
  );
}

/** CQ-010, REPO-005: code lines per file and measured files per folder. */
export function checkLimits(files: SourceFile[], baseline: Baseline): Violation[] {
  return [...fileLines(files, baseline), ...folderFiles(files, baseline)];
}
