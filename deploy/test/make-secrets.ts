// Runs deploy/bin/make-secrets in a temporary folder for the secrets tests.
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export const SCRIPT = join(import.meta.dir, "..", "bin", "make-secrets");

export const GENERATED = [
  "postgres_password",
  "owner_password",
  "hub_password",
  "owner_database_url",
  "hub_database_url",
  "anima_key",
] as const;

const scratchFolders: string[] = [];

/** A new empty folder that `removeScratchFolders` deletes. */
export function scratchFolder(): string {
  const folder = mkdtempSync(join(tmpdir(), "anima-secrets-"));
  scratchFolders.push(folder);
  return folder;
}

export function removeScratchFolders(): void {
  for (const folder of scratchFolders.splice(0)) rmSync(folder, { recursive: true, force: true });
}

/** A folder path that does not exist yet, so make-secrets creates it. */
export function newSecretsPath(): string {
  return join(scratchFolder(), "secrets");
}

export interface RunOptions {
  args?: string[];
  /** Directories put in front of PATH. */
  shimDirectories?: string[];
  /** Replaces PATH entirely. */
  path?: string;
  chown?: "0" | "1";
  stdin?: string;
  /** Makes the script offer the no-echo prompt although stdin is a pipe. */
  assumeTerminal?: boolean;
}

export interface RunResult {
  exitCode: number;
  output: string;
  stdout: string;
  stderr: string;
}

function environment(folder: string, options: RunOptions): Record<string, string | undefined> {
  const path = options.path ?? [...(options.shimDirectories ?? []), process.env["PATH"] ?? ""].join(":");
  return {
    ...process.env,
    PATH: path,
    ANIMA_SECRETS_DIR: folder,
    ANIMA_SECRETS_CHOWN: options.chown ?? "0",
    ...(options.assumeTerminal ? { ANIMA_ASSUME_TERMINAL: "1" } : {}),
  };
}

export function makeSecrets(folder: string, options: RunOptions = {}): RunResult {
  const result = Bun.spawnSync(["/bin/sh", SCRIPT, ...(options.args ?? [])], {
    env: environment(folder, options),
    stdin: options.stdin === undefined ? "ignore" : Buffer.from(options.stdin),
  });
  const stdout = result.stdout.toString();
  const stderr = result.stderr.toString();
  return { exitCode: result.exitCode ?? 1, output: stdout + stderr, stdout, stderr };
}

/** Starts the script with a stdin pipe that stays open, so a test can signal it while it reads. */
export function startMakeSecrets(folder: string, options: RunOptions = {}) {
  return Bun.spawn(["/bin/sh", SCRIPT, ...(options.args ?? [])], {
    env: environment(folder, options),
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
  });
}

export function read(folder: string, name: string): string {
  return readFileSync(join(folder, name), "utf8");
}

/** A folder of executable shell scripts named after the commands they replace. */
export function shimFolder(scripts: Record<string, string>): string {
  const folder = scratchFolder();
  for (const [name, body] of Object.entries(scripts)) {
    const file = join(folder, name);
    writeFileSync(file, `#!/bin/sh\n${body}\n`);
    chmodSync(file, 0o755);
  }
  return folder;
}

/** A folder that links only the named commands, found on the real PATH. */
export function toolsOnly(names: string[]): string {
  const folder = scratchFolder();
  for (const name of names) {
    const found = Bun.which(name);
    if (found === null) throw new Error(`${name} not found on PATH`);
    symlinkSync(found, join(folder, name));
  }
  return folder;
}

export function secretsFolderWith(files: Record<string, string>): string {
  const folder = newSecretsPath();
  mkdirSync(folder, { mode: 0o700 });
  for (const [name, content] of Object.entries(files)) writeFileSync(join(folder, name), content);
  return folder;
}
