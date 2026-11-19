// `bun quality/bin/numbers.ts --check`: fails when a placeholder of a number is in the repository
// (the rule for `dev` and `main`). `--assign`: gives each placeholder its number, in the files of
// the working tree; the merge script runs it (PLN-021).
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { assign, placeholders, type TextFile } from "../src/merge/numbers.ts";
import { REPO_ROOT } from "../src/repository.ts";

function trackedTextFiles(): TextFile[] {
  const listed = Bun.spawnSync(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], { cwd: REPO_ROOT });
  if (listed.exitCode !== 0) throw new Error("git ls-files failed");
  const paths = [...new Set(listed.stdout.toString().split("\0"))].filter((path) => path !== "" && existsSync(join(REPO_ROOT, path)));
  return paths.flatMap((path) => {
    const bytes = readFileSync(join(REPO_ROOT, path));
    // A file with a zero byte is not text; the tests and the source of this rule name placeholders as examples.
    if (bytes.includes(0) || /^quality\/(src\/merge\/|tests\/merge\/)/.test(path)) return [];
    return [{ path, text: bytes.toString("utf8") }];
  });
}

const mode = process.argv[2];
const files = trackedTextFiles();
if (mode === "--check") {
  const found = placeholders(files);
  for (const placeholder of found) console.error(`numbers: ${placeholder} has no number yet`);
  if (found.length > 0) console.error("numbers: a merge into dev gives each its number (bun quality/bin/merge.ts)");
  else console.log("numbers: ok");
  process.exit(found.length > 0 ? 1 : 0);
} else if (mode === "--assign") {
  const done = assign(files);
  for (const { from, to } of done.renamed) renameSync(join(REPO_ROOT, from), join(REPO_ROOT, to));
  for (const file of done.changed) writeFileSync(join(REPO_ROOT, file.path), file.text);
  for (const [placeholder, number] of done.numbers) console.log(`${placeholder} -> ${number}`);
  if (done.numbers.size === 0) console.log("numbers: no placeholder");
} else {
  console.error("usage: bun quality/bin/numbers.ts --check | --assign");
  process.exit(2);
}
