// A strict reader of the Prometheus text format 0.0.4, as the hub's `GET /metrics` writes it (INT-001, INT-002).
// Pure: text in, families and problems out. It never throws. A line with a problem is skipped and the read goes
// on, so the K7 rules still see the rest. A problem names a line number and a fixed kind, never the text of the
// line, so the probe can print it (A-2).
// Strict beyond the format where the hub is: every family has one `# HELP` and one `# TYPE`, the `# TYPE` comes
// before its samples, a family's lines stand together, no series repeats, and the text ends with a line feed.

/** One sample line: the metric name, its labels in written order, and the value. */
export interface Sample {
  name: string;
  labels: [string, string][];
  value: number;
}

/** A family declared by `# HELP` and `# TYPE`, with its samples in written order. */
export interface Family {
  name: string;
  type: string | undefined;
  help: boolean;
  samples: Sample[];
}

/** One problem and its line number (1-based). */
export interface TextProblem {
  problem: string;
  line: number;
}

/** The families of a text and its problems, in line order. */
export interface Exposition {
  families: Map<string, Family>;
  problems: TextProblem[];
}

const METRIC_NAME = /^[a-zA-Z_:][a-zA-Z0-9_:]*/;
const LABEL_NAME = /^[a-zA-Z_][a-zA-Z0-9_]*/;
const METADATA = /^# (HELP|TYPE) ([a-zA-Z_:][a-zA-Z0-9_:]*)(?: (.*))?$/;
const TYPES = new Set(["counter", "gauge", "histogram", "summary", "untyped"]);
const SUFFIXES: Readonly<Record<string, readonly string[]>> = {
  histogram: ["_bucket", "_sum", "_count"],
  summary: ["_sum", "_count"],
};
const NUMBER = /^[+-]?(?:\d+\.?\d*(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?)$/;
const SPECIAL: Readonly<Record<string, number>> = { "+Inf": Infinity, Inf: Infinity, "-Inf": -Infinity, NaN: NaN };
const TIMESTAMP = /^-?\d+$/;
const LABEL_ESCAPES: Readonly<Record<string, string>> = { "\\": "\\", '"': '"', n: "\n" };

class Problem {
  constructor(readonly kind: string) {}
}

/** The value of a sample, or undefined when the text is not a Prometheus float. */
function sampleValue(text: string): number | undefined {
  if (text in SPECIAL) {
    return SPECIAL[text];
  }
  return NUMBER.test(text) ? Number(text) : undefined;
}

/** A quoted label value from `text[start]` (just after the opening quote): the value and the index after its quote. */
function labelValue(text: string, start: number): [string, number] {
  let value = "";
  for (let at = start; at < text.length; at += 1) {
    const char = text.charAt(at);
    if (char === '"') {
      return [value, at + 1];
    }
    if (char === "\\") {
      const escaped = LABEL_ESCAPES[text.charAt(at + 1)];
      if (escaped === undefined) {
        throw new Problem("a label value has an escape the format does not allow");
      }
      value += escaped;
      at += 1;
    } else {
      value += char;
    }
  }
  throw new Problem("a label value is not closed");
}

/** The labels between `{` and `}` from `text[start]` (just after `{`): the pairs and the index after `}`. */
function labelSet(text: string, start: number): [[string, string][], number] {
  const labels: [string, string][] = [];
  let at = start;
  while (text.charAt(at) !== "}") {
    const name = LABEL_NAME.exec(text.slice(at))?.[0];
    if (name === undefined || text.slice(at + name.length, at + name.length + 2) !== '="') {
      throw new Problem("a label is not name=\"value\"");
    }
    if (labels.some(([seen]) => seen === name)) {
      throw new Problem("a label name repeats in one sample");
    }
    const [value, after] = labelValue(text, at + name.length + 2);
    labels.push([name, value]);
    const separator = text.charAt(after);
    if (separator !== "," && separator !== "}") {
      throw new Problem("labels are not separated by commas");
    }
    at = separator === "," ? after + 1 : after;
  }
  return [labels, at + 1];
}

/** One sample line, or a thrown problem. */
function parseSample(line: string): Sample {
  const name = METRIC_NAME.exec(line)?.[0];
  if (name === undefined) {
    throw new Problem("a sample line does not start with a metric name");
  }
  const [labels, after] = line.charAt(name.length) === "{" ? labelSet(line, name.length + 1) : [[], name.length];
  const rest = line.slice(after);
  if (!/^[ \t]/.test(rest)) {
    throw new Problem("a sample has no value");
  }
  const fields = rest.trim().split(/[ \t]+/);
  const value = sampleValue(fields[0] ?? "");
  if (value === undefined || fields.length > 2 || (fields.length === 2 && !TIMESTAMP.test(fields[1] ?? ""))) {
    throw new Problem("a sample value or timestamp is not a number");
  }
  return { name, labels, value };
}

/** The family a sample of `name` belongs to: the name itself, or a histogram or summary name and suffix. */
function familyOf(families: Map<string, Family>, name: string): Family | undefined {
  const exact = families.get(name);
  if (exact !== undefined) {
    return exact;
  }
  for (const family of families.values()) {
    const suffixes = SUFFIXES[family.type ?? ""] ?? [];
    if (suffixes.some((suffix) => name === family.name + suffix)) {
      return family;
    }
  }
  return undefined;
}

/** What one exposition read has seen so far: the families, the family of the last line, the closed groups. */
class Reading {
  readonly families = new Map<string, Family>();
  readonly series = new Set<string>();
  readonly closed = new Set<string>();
  current: string | undefined;

  enter(name: string): void {
    if (name === this.current) {
      return;
    }
    if (this.closed.has(name)) {
      throw new Problem("the lines of a family do not stand together");
    }
    if (this.current !== undefined) {
      this.closed.add(this.current);
    }
    this.current = name;
  }

  metadata(kind: string, name: string, rest: string | undefined): void {
    this.enter(name);
    const family = this.families.get(name) ?? { name, type: undefined, help: false, samples: [] };
    this.families.set(name, family);
    if (kind === "HELP") {
      if (family.help || /\\(?![\\n])/.test(rest ?? "")) {
        throw new Problem(family.help ? "a family has two HELP lines" : "a HELP text has a bad escape");
      }
      family.help = true;
      return;
    }
    if (family.type !== undefined || family.samples.length > 0 || !TYPES.has(rest ?? "")) {
      throw new Problem("a TYPE line repeats, comes after samples or names no known type");
    }
    family.type = rest;
  }

  sample(line: string): void {
    const sample = parseSample(line);
    const family = familyOf(this.families, sample.name);
    if (family?.type === undefined) {
      throw new Problem("a sample belongs to no declared family");
    }
    this.enter(family.name);
    const key = JSON.stringify([sample.name, [...sample.labels].sort(([a], [b]) => (a < b ? -1 : 1))]);
    if (this.series.has(key)) {
      throw new Problem("a series repeats");
    }
    this.series.add(key);
    family.samples.push(sample);
  }
}

function readLine(reading: Reading, line: string): void {
  if (line === "" || (line.startsWith("#") && !/^# (?:HELP|TYPE)(?: |$)/.test(line))) {
    return; // A blank line or a plain comment; the format allows both.
  }
  if (line.startsWith("#")) {
    const match = METADATA.exec(line);
    if (match === null) {
      throw new Problem("a HELP or TYPE line is malformed");
    }
    reading.metadata(match[1] ?? "", match[2] ?? "", match[3]);
    return;
  }
  reading.sample(line);
}

/** Reads a whole exposition. */
export function parseExposition(text: string): Exposition {
  const lines = text.split("\n");
  const problems: TextProblem[] = [];
  if (lines.at(-1) === "") {
    lines.pop();
  } else {
    problems.push({ problem: "the text does not end with a line feed", line: lines.length });
  }
  const reading = new Reading();
  for (const [index, line] of lines.entries()) {
    try {
      if (line.includes("\r")) {
        throw new Problem("a line holds a carriage return");
      }
      readLine(reading, line);
    } catch (error) {
      if (!(error instanceof Problem)) {
        throw error;
      }
      problems.push({ problem: error.kind, line: index + 1 });
    }
  }
  if ([...reading.families.values()].some((family) => !family.help || family.type === undefined)) {
    problems.push({ problem: "a family lacks its HELP or TYPE line", line: lines.length });
  }
  return { families: reading.families, problems };
}
