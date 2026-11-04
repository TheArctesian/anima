import { describe, expect, test } from "bun:test";
import { bannedWordIn, checkNames, wordSegments } from "../src/names.ts";

const signals = (path: string, text = "") => checkNames([{ path, text }]).map((v) => v.signal);

describe("CQ-030 banned names", () => {
  test("a type named UserManager fails, in Rust and TypeScript", () => {
    expect(signals("crates/hub/src/users/user.rs", "pub struct UserManager {}")).toEqual(["banned-type-name"]);
    expect(signals("apps/client/src/lib/user.ts", "export class UserManager {}")).toEqual(["banned-type-name"]);
    expect(signals("a/b.rs", "pub(crate) trait RequestHandler {}\ntype Items = u8;")).toHaveLength(2);
  });

  test("a utils folder fails", () => {
    expect(signals("apps/client/src/utils/format.ts")).toEqual(["banned-folder"]);
    expect(signals("apps/client/src/components/Button.svelte")).toEqual(["banned-folder"]);
  });

  test("device_info.rs fails and metadata.rs passes", () => {
    expect(signals("crates/hub/src/devices/device_info.rs")).toEqual(["banned-file-name"]);
    expect(signals("crates/hub/src/devices/metadata.rs")).toEqual([]);
  });

  test("types named only in comments or Svelte markup do not count", () => {
    expect(signals("a/b.rs", "// struct UserManager\n/* enum Helper */\n/// trait Handler")).toEqual([]);
    expect(signals("a/b.ts", "/* class UserManager {} */\n// interface Helper {}")).toEqual([]);
    expect(signals("a/B.svelte", "<p>\nclass Helper\n</p>\n<script lang=\"ts\">\n// class Util\nclass Handler {}\n</script>")).toEqual(["banned-type-name"]);
  });

  test("segments split on case, underscores, hyphens and dots", () => {
    expect(wordSegments("HTTPServerManager")).toEqual(["http", "server", "manager"]);
    expect(wordSegments("a-b_c.d")).toEqual(["a", "b", "c", "d"]);
    expect(bannedWordIn("Handlers")).toBe("handlers");
    expect(bannedWordIn("string_utils.ts")).toBe("utils");
    expect(bannedWordIn("Metadata")).toBeUndefined();
    expect(bannedWordIn("DeviceId")).toBeUndefined();
  });

  test("docs are skipped and good type names pass", () => {
    expect(signals("docs/common/info.md")).toEqual([]);
    expect(signals("crates/hub/src/devices/device.rs", "pub struct DeviceId(u64);\npub enum Fingerprint {}")).toEqual([]);
  });
});
