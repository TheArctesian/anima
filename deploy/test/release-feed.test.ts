// Behaviour of the release feed the builder writes (UPD-009, UPD-010, DNS-007).
import { describe, expect, test } from "bun:test";
import { buildFeed, checksums, NoPlatformsError, stableName } from "../../apps/client/src/lib/builds/feed.ts";

const BASE = "https://releases.anima.ing/linux-x86_64/0.1.7";
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
      { name: "b.txt", sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824" },
      { name: "a.txt", sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" },
    ]);
    expect(text).toBe(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  a.txt\n" +
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  b.txt\n",
    );
  });
});
