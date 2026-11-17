import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { QUEUE, queueRows, withQueue } from "../src/documents/queue.ts";
import { REGISTER, registerRows, withRows } from "../src/documents/register.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";

// Writes the list of the plans into the register from the header of each document (PLN-020).
const documents = [...new Set(listMeasurableFiles(REPO_ROOT))]
  .filter((path) => /^docs\/.*\.md$/.test(path))
  .sort()
  .map((path) => ({ path, text: readRepoFile(REPO_ROOT, path) }));
const written = withRows(readRepoFile(REPO_ROOT, REGISTER), registerRows(documents));
if (written === undefined) {
  console.error(`register: ${REGISTER} has no generated list`);
  process.exit(1);
}
writeFileSync(join(REPO_ROOT, REGISTER), written);
console.log(`register: wrote ${REGISTER}`);

// The queue reads the register's sources and the state of each header, so write it second.
const queue = withQueue(readRepoFile(REPO_ROOT, QUEUE), queueRows(documents));
if (queue === undefined) {
  console.error(`register: ${QUEUE} has no generated part`);
  process.exit(1);
}
writeFileSync(join(REPO_ROOT, QUEUE), queue);
console.log(`register: wrote ${QUEUE}`);
