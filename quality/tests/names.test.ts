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

describe("CQ1-18 file-name casing", () => {
  const casing = (path: string) => signals(path).filter((signal) => signal === "file-casing");

  test("kebab-case script files pass, with dot-separated suffixes", () => {
    for (const path of [
      "apps/client/src/lib/desktop/this-machine.ts",
      "apps/client/src/lib/checks/bar.fixture.svelte.ts",
      "apps/client/src/lib/hub/socket/fake.ts",
      "apps/client/src/lib/platform/answers.fixture.ts",
      "apps/client/src/lib/updates/desktop.svelte.ts",
      "apps/client/src/lib/forms/modal/modal.svelte.test.ts",
      "apps/client/src/app.d.ts",
      "apps/client/vite.config.ts",
      "deploy/headscale/v2.test.ts",
      "eslint.config.js",
    ]) {
      expect(casing(path)).toEqual([]);
    }
  });

  test("camelCase, snake_case and PascalCase script files fail", () => {
    for (const path of [
      "apps/client/src/lib/desktop/thisMachine.ts",
      "apps/client/src/lib/desktop/process_log.ts",
      "quality/src/Names.ts",
      "apps/client/src/lib/hub/Hub.js",
      "apps/client/src/lib/hub/hub-.ts",
    ]) {
      expect(casing(path)).toEqual(["file-casing"]);
    }
  });

  test("Svelte components are PascalCase; kebab-case components fail", () => {
    expect(casing("apps/client/src/lib/devices/card/DeviceCard.svelte")).toEqual([]);
    expect(casing("apps/client/src/lib/devices/card/device-card.svelte")).toEqual(["file-casing"]);
    expect(casing("apps/client/src/lib/devices/card/deviceCard.svelte")).toEqual(["file-casing"]);
  });

  test("route files, component-named tests and Svelte fixtures are allowed", () => {
    for (const path of [
      "apps/client/src/routes/+page.svelte",
      "apps/client/src/routes/+layout.ts",
      "apps/client/src/routes/devices/[id]/+page.ts",
      "apps/client/src/lib/checks/HeartbeatBar.test.ts",
      "apps/client/src/lib/devices/WakeButton.svelte.test.ts",
      "apps/client/src/lib/sessions/view/filing/hubbed.fixture.svelte",
    ]) {
      expect(casing(path)).toEqual([]);
    }
  });

  test("Rust files and other file types are not checked for casing", () => {
    expect(casing("crates/hub/src/devices/device_state.rs")).toEqual([]);
    expect(casing("docs/Design Spec.md")).toEqual([]);
    expect(casing("apps/client/src/styles/Tokens.css")).toEqual([]);
  });

  test("the casing signal names the file and the expected style", () => {
    const [violation] = checkNames([{ path: "a/thisMachine.ts", text: "" }]);
    expect(violation).toEqual({ path: "a/thisMachine.ts", signal: "file-casing", detail: '"thisMachine.ts" should be kebab-case' });
  });
});
