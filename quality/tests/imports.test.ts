import { describe, expect, test } from "bun:test";
import { checkImports, expandUseTree, modulePathOf, pathsIn, resolvePath } from "../src/imports.ts";

const hub = (path: string, text = "") => ({ path: `crates/hub/src/${path}`, text });
const tree = [hub("devices/mod.rs"), hub("checks/mod.rs"), hub("sessions.rs")];
const run = (...changed: ReturnType<typeof hub>[]) => checkImports([...tree, ...changed]);

describe("REPO-004 store imports", () => {
  test("a domain using another domain's store fails", () => {
    const violations = run(hub("devices/x.rs", "use crate::checks::store;"));
    expect(violations).toEqual([
      { path: "crates/hub/src/devices/x.rs", signal: "foreign-store", detail: "crate::checks::store" },
    ]);
  });

  test("grouped, inline and super paths are found", () => {
    expect(run(hub("devices/x.rs", "use crate::{checks::store, sessions::{a, store}};"))).toHaveLength(2);
    expect(run(hub("devices/x.rs", "fn f() { crate::checks::store::get(); }"))).toHaveLength(1);
    expect(run(hub("devices/inner/x.rs", "use super::super::super::checks::store::Q;"))).toHaveLength(1);
    expect(run(hub("devices/inner/x.rs", "use super::super::checks::store::Q;"))).toEqual([]);
    expect(run(hub("devices/x.rs", "use crate::checks::{self, store};"))).toHaveLength(1);
  });

  test("a domain using its own store, or another domain's public face, passes", () => {
    expect(run(hub("devices/x.rs", "use crate::devices::store::Q;\nuse super::store;\nuse crate::checks::Check;"))).toEqual([]);
  });

  test("comments are ignored and a tree without domains passes", () => {
    expect(run(hub("devices/x.rs", "// use crate::checks::store;"))).toEqual([]);
    expect(checkImports([hub("main.rs", "use crate::schema::build;"), hub("schema.rs")])).toEqual([]);
  });
});

describe("REPO-004 cycles", () => {
  test("a 2-domain cycle fails and reports the path", () => {
    const violations = run(hub("devices/x.rs", "use crate::checks::Check;"), hub("checks/y.rs", "use crate::devices::Device;"));
    expect(violations).toEqual([
      { path: "crates/hub/src/checks", signal: "import-cycle", detail: "checks -> devices -> checks" },
    ]);
  });

  test("a 3-domain cycle fails", () => {
    const violations = run(
      hub("devices/x.rs", "use crate::checks::Check;"),
      hub("checks/y.rs", "use crate::sessions::Session;"),
      hub("sessions.rs", "use crate::devices::Device;"),
    );
    expect(violations.map((v) => v.detail)).toEqual(["checks -> sessions -> devices -> checks"]);
  });

  test("a one-way chain passes", () => {
    expect(run(hub("devices/x.rs", "use crate::checks::Check;"), hub("checks/y.rs", "use crate::sessions::S;"))).toEqual([]);
  });
});

describe("REPO-004 parsing", () => {
  test("use trees expand and super resolves", () => {
    expect(expandUseTree("crate::{a::store, b::{x, store}, c as d}")).toEqual([
      ["crate", "a", "store"], ["crate", "b", "x"], ["crate", "b", "store"], ["crate", "c"],
    ]);
    expect(pathsIn("use a::b; fn f() { self::g(); }")).toEqual([["a", "b"], ["self", "g"]]);
    expect(modulePathOf("crates/hub/src/devices/mod.rs")).toEqual(["devices"]);
    expect(resolvePath(["super", "store"], ["devices", "x"])).toEqual(["devices", "store"]);
  });
});
