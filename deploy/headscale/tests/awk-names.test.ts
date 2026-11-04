import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

// AlmaLinux 10's awk is gawk 5.3, which refuses a keyword or a built-in function as a
// `-v` name ("cannot use gawk builtin `include' as variable name") and gives its special
// variables a meaning. macOS awk accepts all of them, so only this scan catches them here.
// Lists taken from gawk 5.3.0 on almalinux:10: PROCINFO["identifiers"] for the built-ins
// and special variables, and `gawk -v WORD=1` (fatal for each keyword below).
const KEYWORDS =
  "BEGIN BEGINFILE END ENDFILE break case continue default delete do else exit for func function " +
  "getline if in include load namespace next nextfile print printf return switch while";
const BUILTINS =
  "and asort asorti atan2 bindtextdomain close compl cos dcgettext dcngettext exp fflush gensub gsub " +
  "index int isarray length log lshift match mkbool mktime or patsplit rand rshift sin split sprintf " +
  "sqrt srand strftime strtonum sub substr system systime tolower toupper typeof xor";
const SPECIAL =
  "ARGC ARGIND ARGV BINMODE CONVFMT ENVIRON ERRNO FIELDWIDTHS FILENAME FNR FPAT FS FUNCTAB IGNORECASE " +
  "LINT NF NR OFMT OFS ORS PREC PROCINFO RLENGTH ROUNDMODE RS RSTART RT SUBSEP SYMTAB TEXTDOMAIN";
const RESERVED = new Set(`${KEYWORDS} ${BUILTINS} ${SPECIAL}`.split(" "));
const DEPLOY = join(import.meta.dir, "..", "..");

function scripts(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === "node_modules") return [];
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return scripts(path);
    return entry.name.endsWith(".sh") ? [path] : [];
  });
}

/** Each `-v NAME=` and `function NAME(` in a file, with its line number. */
function awkNames(text: string): { name: string; line: number }[] {
  return text.split("\n").flatMap((line, index) =>
    [...line.matchAll(/(?:^|\s)-v\s+([A-Za-z_][A-Za-z0-9_]*)=|\bfunction\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)].map(
      (found) => ({ name: found[1] ?? found[2] ?? "", line: index + 1 }),
    ),
  );
}

describe("awk programs run under gawk (AlmaLinux 10)", () => {
  test("the scan finds -v names and function names, and flags gawk's reserved words", () => {
    const names = awkNames("awk -v include=\"$X\" -v zone=z '\nfunction length(s) { }'");
    expect(names.map((found) => found.name)).toEqual(["include", "zone", "length"]);
    expect(names.filter((found) => RESERVED.has(found.name)).map((found) => found.name)).toEqual(["include", "length"]);
  });

  test("VPS-002: no awk -v name or function name in deploy/ is a gawk keyword, built-in or special variable", () => {
    const files = scripts(DEPLOY);
    expect(files.length).toBeGreaterThan(5);
    const clashes = files.flatMap((file) =>
      awkNames(readFileSync(file, "utf8"))
        .filter((found) => RESERVED.has(found.name))
        .map((found) => `${file.slice(DEPLOY.length + 1)}:${found.line}: ${found.name}`),
    );
    expect(clashes).toEqual([]);
  });
});
