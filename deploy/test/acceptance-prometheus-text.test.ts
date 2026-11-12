// deploy/acceptance/prometheus-text.ts and exposition.ts (criterion 12; INT-001, INT-002, D-24, A-2): the strict
// reader of the hub's Prometheus text and the K7 rules on it. A property test renders random families and reads
// them back; a fuzz test mutates a K7 text and proves the reader never throws and never quotes the text.
import { describe, expect, test } from "bun:test";
import { familiesVerdict, labelValuesVerdict, labelsVerdict, steadyLabel } from "../acceptance/exposition.ts";
import { parseExposition } from "../acceptance/prometheus-text.ts";
import { exposition } from "./acceptance-probe-hub.ts";

const K7_TEXT = exposition(new Set());

describe("INT-002: the reader", () => {
  test("a K7 text reads with no problem and keeps escaped label values", () => {
    const { families, problems } = parseExposition(K7_TEXT);
    expect(problems).toEqual([]);
    expect(families.size).toBe(7);
    const status = families.get("anima_check_status")?.samples[0];
    expect(status?.labels.find(([name]) => name === "check_name")?.[1]).toBe('ping "core"');
    expect(status?.value).toBe(1);
    expect(families.get("anima_check_uptime_ratio")?.samples.map((sample) => sample.labels.at(-1))).toEqual([
      ["window", "1d"],
      ["window", "30d"],
    ]);
  });

  test("each malformed text names its line and a fixed problem", () => {
    const head = "# HELP a A.\n# TYPE a gauge\n";
    const cases: [string, string, number][] = [
      [`${head}a 1`, "the text does not end with a line feed", 3],
      [`${head}b 1\n`, "a sample belongs to no declared family", 3],
      [`${head}a{x="1",x="2"} 1\n`, "a label name repeats in one sample", 3],
      [`${head}a{x="\\t"} 1\n`, "a label value has an escape the format does not allow", 3],
      [`${head}a{x="1} 1\n`, "a label value is not closed", 3],
      [`${head}a{x="1"y="2"} 1\n`, "labels are not separated by commas", 3],
      [`${head}a{x=1} 1\n`, 'a label is not name="value"', 3],
      [`${head}a one\n`, "a sample value or timestamp is not a number", 3],
      [`${head}a 1 2 3\n`, "a sample value or timestamp is not a number", 3],
      [`${head}a\n`, "a sample has no value", 3],
      [`${head}a 1\na 2\n`, "a series repeats", 4],
      [`${head}# TYPE a counter\n`, "a TYPE line repeats, comes after samples or names no known type", 3],
      [`${head}# HELP a again\n`, "a family has two HELP lines", 3],
      [`${head}# HELP b B.\n# TYPE b gauge\na 1\n`, "the lines of a family do not stand together", 5],
      ["# TYPE a gauge\na 1\n", "a family lacks its HELP or TYPE line", 2],
      [`${head}a 1\r\n`, "a line holds a carriage return", 3],
      ["# HELP\n", "a HELP or TYPE line is malformed", 1],
      [`${head}{x="1"} 1\n`, "a sample line does not start with a metric name", 3],
    ];
    for (const [text, problem, line] of cases) {
      expect({ text, problems: parseExposition(text).problems }).toEqual({ text, problems: [{ problem, line }] });
    }
  });

  test("the format's other forms read: comments, blank lines, a timestamp, special values, a trailing comma", () => {
    const text = '# a comment\n\n# HELP a A.\n# TYPE a gauge\na{x="1",} +Inf 1700000000000\na{x="2"} NaN\na -1.5e3\n';
    const { families, problems } = parseExposition(text);
    expect(problems).toEqual([]);
    expect(families.get("a")?.samples.map((sample) => sample.value)).toEqual([Infinity, NaN, -1500]);
  });

});

describe("INT-002: the reader under random input", () => {
  test("property: random families with hostile label values read back as written", () => {
    let state = 20_261_111;
    const next = (bound: number) => (state = (state * 1_103_515_245 + 12_345) % 2_147_483_648) % bound;
    const alphabet = ['"', "\\", "\n", "a", "Z", "{", "}", ",", "=", " ", "é", "#"];
    const escape = (value: string) => value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n");
    for (let round = 0; round < 300; round += 1) {
      const written = Array.from({ length: 1 + next(3) }, (_, family) => ({
        name: `f${String(family)}`,
        samples: Array.from({ length: next(4) }, (_, index) => ({
          labels: [["id", String(index)], ["v", Array.from({ length: next(8) }, () => alphabet[next(alphabet.length)]).join("")]] as [
            string,
            string,
          ][],
          value: next(1000) / 8,
        })),
      }));
      const text = written.map(({ name, samples }) => [`# HELP ${name} H.`, `# TYPE ${name} gauge`,
        ...samples.map((s) => `${name}{${s.labels.map(([k, v]) => `${k}="${escape(v)}"`).join(",")}} ${String(s.value)}`)]
        .join("\n")).join("\n");
      const { families, problems } = parseExposition(`${text}\n`);
      expect(problems).toEqual([]);
      const read = [...families.values()].map(({ name, samples }) => ({ name, samples: samples.map(({ labels, value }) => ({ labels, value })) }));
      expect(read).toEqual(written);
    }
  });

  test("fuzz: a mutated K7 text never throws and no problem quotes the text", () => {
    let state = 7;
    const next = (bound: number) => (state = (state * 1_103_515_245 + 12_345) % 2_147_483_648) % bound;
    const pieces = ['"', "\\", "\n", "{", "}", ",", "=", " ", "#", "\r", "x", "9", "# TYPE ", "\\n"];
    for (let round = 0; round < 2000; round += 1) {
      let text = K7_TEXT;
      for (let edit = 0; edit <= next(4); edit += 1) {
        const at = next(text.length);
        text = text.slice(0, at) + String(pieces[next(pieces.length)]) + text.slice(at + next(3));
      }
      const { problems } = parseExposition(text);
      for (const { problem } of problems) {
        expect(problem).not.toMatch(/pve-01|0190a000|anima_|core|Synthetic/);
      }
    }
  });
});

describe("D-24: the K7 rules", () => {
  const read = (faults: Parameters<typeof exposition>[0]) => parseExposition(exposition(faults)).families;

  test("a K7 text passes every rule", () => {
    const families = read(new Set());
    expect([familiesVerdict(families), labelsVerdict(families), labelValuesVerdict(families)]).toEqual([undefined, undefined, undefined]);
  });

  test("a missing family, an unknown label and a volatile label each fail their rule", () => {
    expect(familiesVerdict(read(new Set(["metrics-family-missing"])))).toBe("the family anima_check_domain_days_left is missing");
    expect(labelsVerdict(read(new Set(["metrics-unknown-label"])))).toBe("a sample of anima_check_status has other labels than D-24 states");
    expect(labelValuesVerdict(read(new Set(["metrics-volatile-label"])))).toContain("a check_name value is not steady");
  });

  test("a label value that changes, or an ID that is not a UUID, is not steady; a device name is", () => {
    const unsteady = ["2026-11-11T09:30:00Z", "2026-11-11", "at 09:30", "1731317400", "1731317400s", "0.25", "5m", "12ms",
      "10.0.0.4", "10.0.0.4:22", "10.0.0.0/24", "fd7a:115c::1", "fd7a:115c::/48", "nas.lan:8080", "Down", "paused"];
    for (const value of unsteady) {
      expect({ value, steady: steadyLabel("device_name", value) }).toEqual({ value, steady: false });
    }
    for (const value of ["pve-01", "nas 2", "printer (office)", "Up stairs", "docker-host", "Ping", "SSH port", "v2 api"]) {
      expect({ value, steady: steadyLabel("device_name", value) }).toEqual({ value, steady: true });
    }
    expect(steadyLabel("device_id", "17")).toBe(false);
    expect(steadyLabel("window", "90d")).toBe(false);
  });
});
