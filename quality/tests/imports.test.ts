import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { HubModulesShapeError, parseHubModules, type HubModules } from "../src/hub-modules.ts";
import { checkImports } from "../src/imports.ts";
import { expandUseTree, mentionsIn, modulePathOf, resolvePath } from "../src/use-paths.ts";

const modules = parseHubModules(readFileSync(join(import.meta.dir, "..", "hub-modules.json"), "utf8"));
const hub = (path: string, text = "") => ({ path: `crates/hub/src/${path}`, text });
const run = (...changed: ReturnType<typeof hub>[]) => checkImports(changed, modules);
const signals = (...changed: ReturnType<typeof hub>[]) => run(...changed).map((v) => `${v.signal} ${v.detail}`);

describe("REPO-004 foreign store", () => {
  test("a domain using another domain's store fails, even on an allowed edge", () => {
    expect(signals(hub("checks/x.rs", "use crate::devices::store;"))).toEqual(["foreign-store crate::devices::store"]);
  });

  test("grouped, inline and super paths are found", () => {
    expect(signals(hub("checks/x.rs", "use crate::{devices::store, events::{a, store}};"))).toHaveLength(2);
    expect(signals(hub("checks/x.rs", "fn f() { crate::devices::store::get(); }"))).toHaveLength(1);
    expect(signals(hub("checks/inner/x.rs", "use super::super::super::devices::store::Q;"))).toHaveLength(1);
    expect(signals(hub("checks/inner/x.rs", "use super::super::devices::store::Q;"))).toEqual([]);
    expect(signals(hub("checks/x.rs", "use crate::devices::{self, store};"))).toHaveLength(1);
  });

  test("aliased and imported names are followed", () => {
    expect(signals(hub("checks/x.rs", "use crate::devices;\nfn f() { devices::store::x(); }"))).toEqual(["foreign-store crate::devices::store"]);
    expect(signals(hub("checks/x.rs", "use crate::devices as d;\nfn f() { d::store::x(); }"))).toHaveLength(1);
    expect(signals(hub("sessions/x.rs", "use crate::access::store as s;\nfn f() { s::x(); }"))).toEqual(["foreign-store crate::access::store"]);
    expect(signals(hub("events/x.rs", "use crate::devices as d;\nfn f() { d::Device::new(); }"))).toEqual(["forbidden-edge events -> devices"]);
  });

  test("own store, another domain's public face and comments pass", () => {
    expect(signals(hub("devices/x.rs", "use crate::devices::store::Q;\nuse super::store;\nuse crate::events::Event;"))).toEqual([]);
    expect(signals(hub("checks/x.rs", "// use crate::devices::store;\n/* crate::devices::store */"))).toEqual([]);
  });
});

describe("REPO-004 literals, globs and duplicates", () => {
  test("a path inside a string literal is not an edge", () => {
    expect(signals(hub("events/x.rs", 'let s = "use crate::sessions::x;";\nlet r = r#"crate::devices::store"#;'))).toEqual([]);
  });

  test("a glob import of another domain fails, even on an allowed edge", () => {
    expect(signals(hub("checks/x.rs", "use crate::devices::*;"))).toEqual(["domain-glob-import checks -> devices::*"]);
    expect(signals(hub("checks/x.rs", "use crate::devices::{Device, *};"))).toHaveLength(1);
    expect(signals(hub("devices/x.rs", "use crate::devices::*;\nuse super::*;"))).toEqual([]);
    expect(signals(hub("checks/x.rs", "use crate::ids::*;"))).toEqual([]);
  });

  test("identical references give one violation", () => {
    expect(signals(hub("checks/x.rs", "use crate::devices::store;\nuse crate::devices::store;"))).toHaveLength(1);
  });
});

describe("REPO-004 allow-list", () => {
  test("a domain using a domain outside its list fails", () => {
    expect(signals(hub("devices/x.rs", "use crate::checks::Check;"))).toEqual(["forbidden-edge devices -> checks"]);
    expect(signals(hub("events/x.rs", "fn f() { crate::sessions::run(); }"))).toEqual(["forbidden-edge events -> sessions"]);
  });

  test("allowed edges pass", () => {
    expect(signals(hub("checks/x.rs", "use crate::devices::Device;\nuse crate::maintenance::Rule;\nuse crate::events::E;"))).toEqual([]);
    expect(signals(hub("sessions/x.rs", "use crate::access::A;\nuse crate::credentials::Vault;"))).toEqual([]);
  });

  test("a leaf module importing a domain fails; importing a leaf passes", () => {
    expect(signals(hub("ids.rs", "use crate::devices::Device;"))).toEqual(["leaf-imports-domain ids -> devices"]);
    expect(signals(hub("devices/x.rs", "use crate::ids::DeviceId;\nuse crate::clock::Clock;"))).toEqual([]);
  });

  test("wiring may import anything, including lib.rs", () => {
    expect(signals(hub("main.rs", "use crate::sessions::run;\nuse crate::checks::Check;"))).toEqual([]);
    expect(signals(hub("lib.rs", "mod devices;\nuse crate::checks::Check;"))).toEqual([]);
    expect(signals(hub("schema.rs", "use crate::devices::store::Q;"))).toEqual(["foreign-store crate::devices::store"]);
  });

  test("a top-level module in no list fails", () => {
    expect(signals(hub("reports/mod.rs"), hub("reports/x.rs"), hub("misc.rs"))).toEqual([
      'unlisted-module "misc" is not in hub-modules.json',
      'unlisted-module "reports" is not in hub-modules.json',
    ]);
  });

  test("a tree with no files passes", () => {
    expect(run()).toEqual([]);
  });

  test("a cyclic allow-list fails", () => {
    const cyclic: HubModules = { leaves: [], wiring: [], domains: { a: ["b"], b: ["a"] }, graphql: {} };
    expect(checkImports([], cyclic).map((v) => v.signal)).toEqual(["allow-list-cycle"]);
  });

  test("the shipped allow-list is acyclic and malformed lists are rejected", () => {
    expect(checkImports([], modules)).toEqual([]);
    expect(() => parseHubModules("{")).toThrow(HubModulesShapeError);
    expect(() => parseHubModules('{"leaves":[],"wiring":[],"domains":{"a":"b"}}')).toThrow(HubModulesShapeError);
  });
});

describe("REPO-004 cycles", () => {
  const loose: HubModules = {
    leaves: [],
    wiring: [],
    domains: { a: ["b", "c"], b: ["a", "c"], c: ["a"] },
    graphql: {},
  };
  const cycles = (...files: ReturnType<typeof hub>[]) =>
    checkImports(files, loose).filter((v) => v.signal === "import-cycle");

  test("a 2-domain cycle in use reports the path", () => {
    expect(cycles(hub("a/x.rs", "use crate::b::B;"), hub("b/y.rs", "use crate::a::A;"))).toEqual([
      { path: "crates/hub/src/a", signal: "import-cycle", detail: "a -> b -> a" },
    ]);
  });

  test("a 3-domain cycle in use fails", () => {
    const found = cycles(hub("a/x.rs", "use crate::b::B;"), hub("b/y.rs", "use crate::c::C;"), hub("c.rs", "use crate::a::A;"));
    expect(found.map((v) => v.detail)).toEqual(["a -> b -> c -> a"]);
  });

  test("a one-way chain passes", () => {
    expect(cycles(hub("a/x.rs", "use crate::b::B;"), hub("b/y.rs", "use crate::c::C;"))).toEqual([]);
  });
});

describe("REPO-004 parsing", () => {
  test("use trees expand with aliases and super resolves", () => {
    expect(expandUseTree("crate::{a::store, b::{x, store}, c as d}")).toEqual([
      { path: ["crate", "a", "store"] },
      { path: ["crate", "b", "x"] },
      { path: ["crate", "b", "store"] },
      { path: ["crate", "c"], alias: "d" },
    ]);
    expect(mentionsIn("use a::b; fn f() { self::g(); }").inline).toEqual([["self", "g"]]);
    expect(modulePathOf("devices/mod.rs")).toEqual(["devices"]);
    expect(resolvePath(["super", "store"], ["devices", "x"])).toEqual(["devices", "store"]);
  });
});

describe("R-011 graphql composition edges", () => {
  test("a domain's graphql module may use the extra edges; its other files may not", () => {
    expect(signals(hub("devices/graphql.rs", "use crate::checks::device_status;\nuse crate::sessions::host_key;"))).toEqual([]);
    expect(signals(hub("devices/rules.rs", "use crate::checks::device_status;"))).toEqual(["forbidden-edge devices -> checks"]);
  });

  test("graphql edges never permit a foreign store", () => {
    expect(signals(hub("devices/graphql.rs", "use crate::checks::store;"))).toEqual(["foreign-store crate::checks::store"]);
  });

  test("graphql edges do not make an import cycle", () => {
    const changed = [hub("devices/graphql.rs", "use crate::checks::x;"), hub("checks/x.rs", "use crate::devices::Device;")];
    expect(run(...changed)).toEqual([]);
  });

  test.each(["null", "[]", "3", '"text"', '{"domains":[]}'])("a hub-modules root of %s is rejected", (text) => {
    expect(() => parseHubModules(text)).toThrow(HubModulesShapeError);
  });

  test("a graphql entry must name a domain", () => {
    expect(() => parseHubModules('{"leaves":[],"wiring":[],"domains":{},"graphql":{"nope":[]}}')).toThrow(HubModulesShapeError);
  });
});
