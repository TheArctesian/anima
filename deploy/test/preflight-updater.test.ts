// deploy/bin/preflight on the updater key pair and the secret files' owner and group (R-116, D-44,
// S-3, S-4, C-7, C-9). Nothing here needs Docker.
import { describe, expect, test } from "bun:test";
import { chmodSync, chownSync, renameSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { finish } from "./finished-process.ts";
import { GROUP, OWNER, SCRIPT, pair, preflight, secrets, withFolder, withPair, write } from "./preflight-fixture.ts";

describe("preflight updater key pair (R-116, D-44)", () => {
  test.each([[pair], [pair.slice(0, 1)], [pair.slice(1)]])("D-44 warns but passes when %p is absent, for the hub starts with publishing off", async (absent) => {
    await withFolder(async (folder) => {
      withPair(folder);
      for (const name of absent) rmSync(join(folder, name));
      const result = await secrets(folder);
      expect(result.ok).toBe(true);
      expect(result.message).toContain(absent.length === 2 ? "warning: no updater key pair" : `warning: ${join(folder, absent[0] ?? "")} is missing`);
      expect(result.message).toContain("publishing off");
    });
  });

  test("D-44 is silent with the pair, accepts an empty password for a key with none and refuses an empty key", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      expect(await secrets(folder)).toEqual({ ok: true, message: "" });
      write(folder, "updater/updater_key_password", "");
      expect(await secrets(folder)).toEqual({ ok: true, message: "" });
      write(folder, "updater/updater_key", "");
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("updater_key is empty");
    });
  });

  test.each([
    ["updater/updater_key", 0o600],
    ["updater/updater_key", 0o444],
    ["updater/updater_key_password", 0o600],
    ["updater/updater_key_password", 0o400],
  ])("D-44 refuses %s with mode %o and prints no value", async (name, mode) => {
    await withFolder(async (folder) => {
      withPair(folder);
      write(folder, name, "updater-value-1", mode);
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(name);
      expect(result.message).toContain("mode 0440");
      expect(result.message).not.toContain("updater-value-1");
    });
  });

});

describe("preflight updater folder rights and contents (S-3, C-7, D-44)", () => {
  test.skipIf(process.getuid?.() === 0)("S-3 refuses a folder it cannot search and says to run with sudo, never that the pair is absent", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      chmodSync(join(folder, "updater"), 0o640);
      const result = await secrets(folder);
      chmodSync(join(folder, "updater"), 0o750);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("run preflight with sudo");
      expect(result.message).not.toContain("no updater key pair");
    });
  });

  test("S-3 refuses an updater folder that is not 0750 and names it", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      chmodSync(join(folder, "updater"), 0o755);
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain("mode 0750");
    });
  });

  // Another group the test's user is in, so one file can leave the stand-in hub group.
  const otherGroup = (process.getgroups?.() ?? []).find((group) => group !== process.getgid?.());

  test.skipIf(otherGroup === undefined).each(["updater", "updater/updater_key", "updater/updater_key_password", "r2_secret_access_key"])(
    "S-3 refuses %s outside the hub's group and prints no value",
    async (name) => {
      await withFolder(async (folder) => {
        withPair(folder);
        write(folder, "updater/updater_key", "updater-value-1");
        chownSync(join(folder, name), process.getuid?.() ?? 0, otherGroup ?? 0);
        const result = await secrets(folder);
        expect(result.ok).toBe(false);
        expect(result.message).toContain(`${name} is not`);
        expect(result.message).toContain(`group ${GROUP}`);
        expect(result.message).not.toContain("updater-value-1");
      });
    },
  );

  test("C-7 warns of each other name in the updater folder but passes, and prints no value", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      write(folder, "updater/updater_key.bak", "updater-value-1");
      write(folder, "updater/.updater_key.swp", "updater-value-2");
      const result = await secrets(folder);
      expect(result.ok).toBe(true);
      expect(result.message).toContain("'updater_key.bak', which is not part of the updater key pair");
      expect(result.message).toContain("'.updater_key.swp', which is not part of the updater key pair");
      expect(result.message).not.toContain("updater-value-");
    });
  });
});

describe("preflight secrets owner and group (S-4)", () => {
  test("S-4 a .env sourced as the README does cannot move the folder, owner or hub group", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      const env = join(folder, "dot-env");
      writeFileSync(env, `ANIMA_SECRETS_OWNER=${OWNER}\nANIMA_SECRETS_GROUP=${GROUP}\nANIMA_SECRETS_DIR=${folder}\n`);
      const run = (check: string) => finish(["sh", "-c", `set -a; . "${env}"; set +a; ${check}`]);
      const named = await run(`sh "${SCRIPT}" secrets "${folder}"`);
      expect(named.exitCode).toBe(1);
      expect(named.stderr).toContain("is not in the hub's group 10001");
      const unnamed = await run(`sh "${SCRIPT}" secrets`);
      expect(unnamed.exitCode).toBe(1);
      expect(unnamed.stderr).not.toContain(folder);
      expect(unnamed.stderr).toContain("/bin/../secrets/");
    });
  });

  test("S-4 refuses an owner or group that is not a number", async () => {
    await withFolder(async (folder) => {
      withPair(folder);
      const result = await preflight("secrets", folder, OWNER, "hub");
      expect(result.ok).toBe(false);
      expect(result.message).toContain("must be numbers");
    });
  });
});

describe("preflight updater links (C-9)", () => {
  test.each([
    ["the folder", (folder: string) => {
      renameSync(join(folder, "updater"), join(folder, "real"));
      symlinkSync(join(folder, "real"), join(folder, "updater"));
    }, "updater is a symbolic link"],
    ["the key", (folder: string) => {
      renameSync(join(folder, "updater/updater_key"), join(folder, "key"));
      symlinkSync(join(folder, "key"), join(folder, "updater/updater_key"));
    }, "updater_key is a symbolic link"],
    ["a dangling key", (folder: string) => {
      rmSync(join(folder, "updater/updater_key"));
      symlinkSync(join(folder, "gone"), join(folder, "updater/updater_key"));
    }, "updater_key is a symbolic link"],
    ["another name", (folder: string) => {
      symlinkSync(join(folder, "gone"), join(folder, "updater/.key.swp"));
    }, ".key.swp is a symbolic link"],
  ])("C-9 refuses %s as a symbolic link, never as absent", async (_, link, said) => {
    await withFolder(async (folder) => {
      withPair(folder);
      link(folder);
      const result = await secrets(folder);
      expect(result.ok).toBe(false);
      expect(result.message).toContain(said);
      expect(result.message).not.toContain("no updater key pair");
    });
  });
});
