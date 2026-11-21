import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { MOCKUPS, mockupTable, withMockupTable } from "../src/documents/mockup-records.ts";
import { QUEUE, queueRows, withQueue } from "../src/documents/queue.ts";
import { REGISTER, registerRows, withRows } from "../src/documents/register.ts";
import { settleGenerated } from "../src/merge/finish.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";

// `--settle`: a merge that stopped only on generated files and the mockups is settled first
// (the merge of a branch with `dev` in a run of the gates); any other conflict ends with exit 1.
if (process.argv.includes("--settle")) {
  const reason = settleGenerated(REPO_ROOT);
  if (reason !== undefined) {
    console.error(`register: the merge needs a person: ${reason}`);
    process.exit(1);
  }
}

// Writes the list of the plans into the register from the header of each document (PLN-020).
const documents = [...new Set(listMeasurableFiles(REPO_ROOT))]
  .filter((path) => /^docs\/.*\.md$/.test(path))
  .sort()
  .map((path) => ({ path, text: readRepoFile(REPO_ROOT, path) }));

/** The text of a generated file, or an error line and exit 1 when the file is missing. */
function existing(path: string): string {
  if (existsSync(join(REPO_ROOT, path))) return readRepoFile(REPO_ROOT, path);
  console.error(`register: ${path} does not exist; create it with the two marker comments of its generated part`);
  process.exit(1);
}

const written = withRows(existing(REGISTER), registerRows(documents));
if (written === undefined) {
  console.error(`register: ${REGISTER} has no generated list`);
  process.exit(1);
}
writeFileSync(join(REPO_ROOT, REGISTER), written);
console.log(`register: wrote ${REGISTER}`);

// The queue reads the register's sources and the state of each header, so write it second.
const queue = withQueue(existing(QUEUE), queueRows(documents));
if (queue === undefined) {
  console.error(`register: ${QUEUE} has no generated part`);
  process.exit(1);
}
writeFileSync(join(REPO_ROOT, QUEUE), queue);
console.log(`register: wrote ${QUEUE}`);

// The index of the mockups is written from the record of each mockup (PLN-019).
const mockups = withMockupTable(existing(MOCKUPS), mockupTable(documents));
if (mockups === undefined) {
  console.error(`register: ${MOCKUPS} has no generated table`);
  process.exit(1);
}
writeFileSync(join(REPO_ROOT, MOCKUPS), mockups);
console.log(`register: wrote ${MOCKUPS}`);
