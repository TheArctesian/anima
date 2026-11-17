import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { QUEUE, queueRows, withQueue } from "../src/documents/queue.ts";
import { REGISTER, registerRows, withRows } from "../src/documents/register.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";

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
