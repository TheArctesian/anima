// Makes the Anima roles and the database on the throwaway server, once per server (R-009, R-019).
// Roles are cluster-wide, so every test file that needs them calls `installRoles()` and gets the
// same result, whichever file runs first.
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { psql, scalar, serverDirectory } from "./postgres.ts";

export const ROOT = join(import.meta.dir, "..", "..");
export const ROLES_SQL = join(ROOT, "deploy", "postgres", "roles.sql");
export const MIGRATIONS = join(ROOT, "crates", "hub", "migrations");

export interface InstalledRoles {
  secretsFolder: string;
  /** Environment that makes roles.sql read the generated password files. */
  passwordEnv: Record<string, string>;
}

/** Applies every hub migration, in order, to `database` as anima_owner. */
export function migrate(database = "anima"): void {
  const files = readdirSync(MIGRATIONS).filter((name) => name.endsWith(".sql")).sort();
  for (const file of files) {
    const applied = psql(["-f", join(MIGRATIONS, file)], { user: "anima_owner", database });
    if (applied.exitCode !== 0) throw new Error(`migration ${file} failed: ${applied.stderr}`);
  }
}

/** Makes the secrets, roles, database and schema if this server does not have them yet. */
export function installRoles(): InstalledRoles {
  const secretsFolder = join(serverDirectory(), "secrets");
  const made = Bun.spawnSync(["/bin/sh", join(ROOT, "deploy", "bin", "make-secrets"), secretsFolder], {
    env: { ...process.env, ANIMA_SECRETS_CHOWN: "0" },
    stdin: "ignore",
  });
  if (made.exitCode !== 0) throw new Error(`make-secrets failed: ${made.stderr.toString()}`);
  const passwordEnv = {
    ANIMA_OWNER_PASSWORD_FILE: join(secretsFolder, "owner_password"),
    ANIMA_HUB_PASSWORD_FILE: join(secretsFolder, "hub_password"),
  };
  if (scalar("select count(*) from pg_roles where rolname = 'anima_owner'") === "0") {
    const result = psql(["-f", ROLES_SQL], { env: passwordEnv });
    if (result.exitCode !== 0) throw new Error(`roles.sql failed: ${result.stderr}`);
    migrate();
  }
  return { secretsFolder, passwordEnv };
}
