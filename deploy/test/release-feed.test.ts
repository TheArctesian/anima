// Behaviour of the release feed (UPD-009, UPD-010, DNS-007) and of the CLI that writes it.
import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SignedVersionError, assertSignedVersion, signedVersion } from "../release/signed-version.ts";
import { buildFeed, checksums, NoPlatformsError, stableName } from "../release/feed.ts";

const BASE = "https://github.com/TheArctesian/anima/releases/download/v0.1.7";
const DATE = new Date("2026-11-04T10:20:30.456Z");
const macos = { file: "Anima.app.tar.gz", signature: "SIG-MAC\n" };
const linux = { file: "Anima_0.1.7_amd64.AppImage", signature: "SIG-LINUX" };

function feedOf(built: Parameters<typeof buildFeed>[0]["built"]) {
  return buildFeed({ version: "0.1.7", baseUrl: BASE, date: DATE, built });
}

describe("latest.json (UPD-009, DNS-007)", () => {
  test("UPD-009 lists both Mac architectures with the universal file and Linux with the AppImage", () => {
    const feed = feedOf({ macos, linux });
    expect(feed.version).toBe("0.1.7");
    expect(feed.pub_date).toBe("2026-11-04T10:20:30Z");
    expect(feed.platforms).toEqual({
      "darwin-aarch64": { signature: "SIG-MAC", url: `${BASE}/Anima.app.tar.gz` },
      "darwin-x86_64": { signature: "SIG-MAC", url: `${BASE}/Anima.app.tar.gz` },
      "linux-x86_64": { signature: "SIG-LINUX", url: `${BASE}/Anima_0.1.7_amd64.AppImage` },
    });
  });

  test("UPD-009 a platform that did not build is not listed", () => {
    expect(Object.keys(feedOf({ macos }).platforms).sort()).toEqual(["darwin-aarch64", "darwin-x86_64"]);
    expect(Object.keys(feedOf({ linux }).platforms)).toEqual(["linux-x86_64"]);
  });

  test("UPD-009 a platform whose file has no signature is not listed", () => {
    const feed = feedOf({ macos, linux: { file: linux.file, signature: "  \n" } });
    expect(feed.platforms["linux-x86_64"]).toBeUndefined();
  });

  test("UPD-009 a release with no platform is an error", () => {
    expect(() => feedOf({})).toThrow(NoPlatformsError);
  });

  test("DNS-007 file names are URL-encoded", () => {
    const feed = feedOf({ linux: { file: "Anima 0.1.7#1.AppImage", signature: "s" } });
    expect(feed.platforms["linux-x86_64"]?.url).toBe(`${BASE}/Anima%200.1.7%231.AppImage`);
  });
});

describe("stable names and sums (UPD-010)", () => {
  test("UPD-010 the DMG and the AppImage get their stable names, other files none", () => {
    expect(stableName("Anima_0.1.7_universal.dmg")).toBe("Anima.dmg");
    expect(stableName("Anima_0.1.7_amd64.AppImage")).toBe("Anima-x86_64.AppImage");
    expect(stableName("Anima.app.tar.gz")).toBeUndefined();
    expect(stableName("Anima_0.1.7_amd64.AppImage.sig")).toBeUndefined();
  });

  test("UPD-010 SHA256SUMS has one '<hex>  <name>' line per file, sorted by name", () => {
    const text = checksums([
      { name: "b.txt", bytes: new TextEncoder().encode("hello") },
      { name: "a.txt", bytes: new Uint8Array() },
    ]);
    expect(text).toBe(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  a.txt\n" +
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  b.txt\n",
    );
  });
});

const script = join(import.meta.dir, "..", "release", "make-feed.ts");
const roots: string[] = [];
afterAll(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true });
});

function folder(): string {
  const root = mkdtempSync(join(tmpdir(), "anima-feed-"));
  roots.push(root);
  return root;
}

function put(root: string, path: string, text: string): void {
  mkdirSync(join(root, path, ".."), { recursive: true });
  writeFileSync(join(root, path), text);
}

function run(root: string, version = "0.1.7") {
  return Bun.spawnSync(
    ["bun", script, "--version", version, "--tag", "v0.1.7", "--artifacts", join(root, "artifacts"), "--out", join(root, "out")],
    { stdout: "pipe", stderr: "pipe" },
  );
}

/** A `.sig` file as the Tauri CLI writes it: base64 of minisign text with a trusted comment. */
function sigFile(comment: string | undefined): string {
  const lines = ["untrusted comment: signature from tauri secret key", "RUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc="];
  if (comment !== undefined) lines.push(`trusted comment: ${comment}`);
  lines.push("DDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==");
  return Buffer.from(`${lines.join("\n")}\n`).toString("base64");
}
const signedFor = (version: string, file = "payload") => sigFile(`timestamp:1791649051\tfile:${file}\tversion:${version}`);
const MAC_SIG = signedFor("0.1.7", "mac");
const LINUX_SIG = signedFor("0.1.7", "linux");
const sha = (text: string) => new Bun.CryptoHasher("sha256").update(text).digest("hex");
const read = (root: string, name: string) => readFileSync(join(root, "out", name), "utf8");

function bothPlatforms(): string {
  const root = folder();
  put(root, "artifacts/macos/x/Anima.app.tar.gz", "app");
  put(root, "artifacts/macos/x/Anima.app.tar.gz.sig", `${MAC_SIG}\n`);
  put(root, "artifacts/macos/y/Anima_0.1.7_universal.dmg", "dmg");
  put(root, "artifacts/linux/Anima_0.1.7_amd64.AppImage", "img");
  put(root, "artifacts/linux/Anima_0.1.7_amd64.AppImage.sig", `${LINUX_SIG}\n`);
  expect(run(root).exitCode).toBe(0);
  return root;
}

describe("make-feed CLI with both platforms (UPD-009, UPD-010)", () => {
  test("UPD-010 writes every file, the stable names, SHA256SUMS and latest.json", () => {
    const root = bothPlatforms();
    expect(readdirSync(join(root, "out")).sort()).toEqual([
      "Anima-x86_64.AppImage",
      "Anima.app.tar.gz",
      "Anima.app.tar.gz.sig",
      "Anima.dmg",
      "Anima_0.1.7_amd64.AppImage",
      "Anima_0.1.7_amd64.AppImage.sig",
      "Anima_0.1.7_universal.dmg",
      "SHA256SUMS",
      "latest.json",
    ]);
  });

  test("UPD-009 latest.json holds the version, the .sig texts and the /v0.1.7/ URLs", () => {
    const feed = JSON.parse(read(bothPlatforms(), "latest.json")) as Record<string, unknown>;
    const base = "https://github.com/TheArctesian/anima/releases/download/v0.1.7";
    const actual: Record<string, unknown> = { ...feed, pub_date: "" };
    expect(actual).toEqual({
      version: "0.1.7",
      notes: "Anima 0.1.7",
      pub_date: "",
      platforms: {
        "darwin-aarch64": { signature: MAC_SIG, url: `${base}/Anima.app.tar.gz` },
        "darwin-x86_64": { signature: MAC_SIG, url: `${base}/Anima.app.tar.gz` },
        "linux-x86_64": { signature: LINUX_SIG, url: `${base}/Anima_0.1.7_amd64.AppImage` },
      },
    });
  });

  test("UPD-010 SHA256SUMS lists the content hash of each file except latest.json", () => {
    const sums = read(bothPlatforms(), "SHA256SUMS").trimEnd().split("\n");
    expect(sums).toHaveLength(7);
    expect(sums).toContain(`${sha("dmg")}  Anima.dmg`);
    expect(sums).toContain(`${sha("img")}  Anima-x86_64.AppImage`);
    expect(sums).toContain(`${sha("app")}  Anima.app.tar.gz`);
    expect(sums.join("\n")).not.toContain("latest.json");
  });
});

describe("make-feed CLI with missing pieces (UPD-009)", () => {
  test("UPD-009 a missing platform folder leaves the platform out", () => {
    const root = folder();
    put(root, "artifacts/linux/a.AppImage", "img");
    put(root, "artifacts/linux/a.AppImage.sig", LINUX_SIG);
    expect(run(root).exitCode).toBe(0);
    const feed = JSON.parse(read(root, "latest.json")) as { platforms: object };
    expect(Object.keys(feed.platforms)).toEqual(["linux-x86_64"]);
  });

  test("UPD-009 a bundle with no .sig is left out and a warning is printed", () => {
    const root = folder();
    put(root, "artifacts/macos/Anima.app.tar.gz", "app");
    put(root, "artifacts/linux/a.AppImage", "img");
    put(root, "artifacts/linux/a.AppImage.sig", LINUX_SIG);
    const result = run(root);
    expect(result.exitCode).toBe(0);
    expect(result.stderr.toString()).toContain("Anima.app.tar.gz has no .sig");
    const feed = JSON.parse(read(root, "latest.json")) as { platforms: object };
    expect(Object.keys(feed.platforms)).toEqual(["linux-x86_64"]);
  });

  test("UPD-009 no artifact at all fails and writes no latest.json", () => {
    const root = folder();
    expect(run(root).exitCode).not.toBe(0);
    expect(readdirSync(join(root, "out"))).not.toContain("latest.json");
  });

  test("UPD-009 a version with a leading v is refused", () => {
    expect(run(folder(), "v0.1.7").exitCode).toBe(2);
  });
});

describe("signed version (UPD-002, UPD-006)", () => {
  test("UPD-002 a signature made for this version is accepted", () => {
    expect(signedVersion(signedFor("0.1.7"))).toBe("0.1.7");
    expect(() => assertSignedVersion("a", signedFor("0.1.7"), "0.1.7")).not.toThrow();
  });

  test("UPD-006 a signature with no version field is refused", () => {
    const sig = sigFile("timestamp:1791649051\tfile:payload");
    expect(signedVersion(sig)).toBeUndefined();
    expect(() => assertSignedVersion("a", sig, "0.1.7")).toThrow(SignedVersionError);
  });

  test("UPD-006 a signature with no trusted comment is refused", () => {
    expect(() => assertSignedVersion("a", sigFile(undefined), "0.1.7")).toThrow(SignedVersionError);
  });

  test("UPD-006 a signature made for another version is refused", () => {
    expect(() => assertSignedVersion("a", signedFor("0.1.6"), "0.1.7")).toThrow("signed for version 0.1.6, expected 0.1.7");
  });

  test("UPD-006 a version that only starts with the expected one is refused", () => {
    expect(() => assertSignedVersion("a", signedFor("0.1.70"), "0.1.7")).toThrow(SignedVersionError);
  });
});

describe("make-feed CLI refuses a wrong signed version (UPD-006)", () => {
  test("UPD-006 a .sig for another version fails the run and writes no latest.json", () => {
    const root = folder();
    put(root, "artifacts/linux/a.AppImage", "img");
    put(root, "artifacts/linux/a.AppImage.sig", `${signedFor("0.1.6")}\n`);
    const result = run(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("expected 0.1.7");
    expect(existsSync(join(root, "out", "latest.json"))).toBe(false);
  });
});
