// Behaviour of the release feed (UPD-009, UPD-010, DNS-007) and of the CLI that writes it.
import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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

describe("make-feed CLI (UPD-009, UPD-010)", () => {
  const script = join(import.meta.dir, "..", "release", "make-feed.ts");

  function run(artifacts: string, out: string) {
    return Bun.spawnSync(
      ["bun", script, "--version", "0.1.7", "--tag", "v0.1.7", "--artifacts", artifacts, "--out", out],
      { stdout: "pipe", stderr: "pipe" },
    );
  }

  test("UPD-009 a missing platform folder leaves the platform out; files, stable names and sums are written", () => {
    const root = mkdtempSync(join(tmpdir(), "anima-feed-"));
    const mac = join(root, "artifacts", "macos", "nested");
    mkdirSync(mac, { recursive: true });
    writeFileSync(join(mac, "Anima.app.tar.gz"), "app");
    writeFileSync(join(mac, "Anima.app.tar.gz.sig"), "SIG-MAC\n");
    writeFileSync(join(mac, "Anima_0.1.7_universal.dmg"), "dmg");
    const out = join(root, "out");

    const result = run(join(root, "artifacts"), out);
    expect(result.exitCode).toBe(0);

    expect(readdirSync(out).sort()).toEqual([
      "Anima.app.tar.gz",
      "Anima.app.tar.gz.sig",
      "Anima.dmg",
      "Anima_0.1.7_universal.dmg",
      "SHA256SUMS",
      "latest.json",
    ]);
    const feed = JSON.parse(readFileSync(join(out, "latest.json"), "utf8")) as { platforms: object };
    expect(Object.keys(feed.platforms).sort()).toEqual(["darwin-aarch64", "darwin-x86_64"]);
    const sums = readFileSync(join(out, "SHA256SUMS"), "utf8");
    expect(sums).toContain("  Anima.dmg\n");
    expect(sums).not.toContain("latest.json");
  });

  test("UPD-009 no artifact at all fails and writes no latest.json", () => {
    const root = mkdtempSync(join(tmpdir(), "anima-feed-"));
    const out = join(root, "out");
    expect(run(join(root, "artifacts"), out).exitCode).not.toBe(0);
    expect(readdirSync(out)).not.toContain("latest.json");
  });
});
