// Installs the Anima roles and the `anima` database once per throwaway server, whichever test
// file runs first (roles are cluster-wide). Later callers get the same password files.
import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { psql, serverDirectory } from "./postgres.ts";

export const ROOT = join(import.meta.dir, "..", "..");
export const ROLES_SQL = join(ROOT, "deploy", "postgres", "roles.sql");
export const MIGRATIONS = join(ROOT, "crates", "hub", "migrations");

export interface InstalledRoles {
  secretsFolder: string;
  /** Environment that roles.sql reads its passwords from. */
  passwordEnv: Record<string, string>;
}

/** Applies every hub migration, in order, to `database` as the owner role. */
export function migrate(database: string): void {
  for (const file of readdirSync(MIGRATIONS).filter((name) => name.endsWith(".sql")).sort()) {
    const applied = psql(["-f", join(MIGRATIONS, file)], { user: "anima_owner", database });
    if (applied.exitCode !== 0) throw new Error(`migration ${file} failed: ${applied.stderr}`);
  }
}

export function installRoles(): InstalledRoles {
  // The folder lives in the server directory, so it goes away with the server.
  const secretsFolder = join(serverDirectory(), "anima-secrets");
  const passwordEnv = {
    ANIMA_OWNER_PASSWORD_FILE: join(secretsFolder, "owner_password"),
    ANIMA_HUB_PASSWORD_FILE: join(secretsFolder, "hub_password"),
  };
  const marker = join(serverDirectory(), "anima-roles-installed");
  if (existsSync(marker)) return { secretsFolder, passwordEnv };

  mkdirSync(secretsFolder, { mode: 0o700 });
  const made = Bun.spawnSync(["sh", join(ROOT, "deploy", "bin", "make-secrets"), secretsFolder], {
    env: { ...process.env, ANIMA_SECRETS_CHOWN: "0" },
    stdin: "ignore",
  });
  if (made.exitCode !== 0) throw new Error(`make-secrets failed: ${made.stderr.toString()}`);
  const result = psql(["-f", ROLES_SQL], { env: passwordEnv });
  if (result.exitCode !== 0) throw new Error(`roles.sql failed: ${result.stderr}`);
  migrate("anima");
  writeFileSync(marker, "installed\n");
  return { secretsFolder, passwordEnv };
}
