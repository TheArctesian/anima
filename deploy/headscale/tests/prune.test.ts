import { afterEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { fixture, PASSWORD, removeFixtures, resticCommands, run, snapshotList, type Run } from "./backup-fixture.ts";

// VPS-006 / D-12: a compromised VPS may append junk snapshots (future-dated, or many per day)
// to push real snapshots out of the retention policy. Retention is `--keep-within 1y` only, and
// prune.sh refuses future-dated or flooded lists.

const NOW = "2026-11-04T12:00:00Z";
const NOW_SECONDS = String(Date.parse(NOW) / 1000);

afterEach(removeFixtures);

/** Daily snapshots at 03:15 UTC for the `days` days before NOW. */
function dailyTimes(days: number): string[] {
  return Array.from({ length: days }, (_, index) =>
    new Date(Date.parse("2026-11-04T03:15:12Z") - index * 86_400_000).toISOString());
}

function prune(times: string[], env: Record<string, string> = {}): Run {
  const root = fixture();
  return run("prune.sh", { FAKE_SNAPSHOTS: snapshotList(root, times), PRUNE_NOW: NOW_SECONDS, ...env }, root);
}

/** The retention flags of a forget call and their values. */
function keepFlags(argv: string[] | undefined): string[] {
  return (argv ?? []).flatMap((arg, index) => (arg.startsWith("--keep-") ? [arg, argv?.[index + 1] ?? ""] : []));
}

function commandNames(result: Run): string[] {
  return resticCommands(result.calls).map((argv) => argv.find((arg) => !arg.startsWith("-") && !arg.includes("/")) ?? "");
}

describe("prune.sh", () => {
  test("VPS-006 checks the snapshot list, then forgets everything older than a year grouped by host and tag, then checks", () => {
    const result = prune(dailyTimes(40));
    expect(result.code).toBe(0);
    expect(commandNames(result)).toEqual(["snapshots", "forget", "check"]);
    const [snapshots, forget] = resticCommands(result.calls);
    expect(snapshots).toEqual(expect.arrayContaining(["snapshots", "--json", "--host", "anima-vps", "--tag", "headscale"]));
    expect(forget?.slice(0, 4)).toEqual([
      "--repo", join(result.root, "repository"), "--password-file", join(result.root, "secrets", "restic-password"),
    ]);
    expect(forget).toEqual(expect.arrayContaining([
      "forget", "--host", "anima-vps", "--tag", "headscale", "--group-by", "host,tags",
      "--keep-within", "1y", "--prune",
    ]));
    expect(keepFlags(forget)).toEqual(["--keep-within", "1y"]);
    expect(result.out).not.toContain(PASSWORD);
  });

  test("VPS-006 past-dated junk snapshots cannot evict a real snapshot from the last year", () => {
    const real = dailyTimes(360);
    // One junk snapshot beside each real one (two per day, under the flood guard), dated
    // later that day so a per-week or per-month rule would keep the junk, plus two per day
    // before the history began.
    const junk = [
      ...real.map((time) => time.replace("T03:15:12", "T23:59:00")),
      ...dailyTimes(400).slice(360).flatMap((time) => [time.replace("T03:15:12", "T22:00:00"), time]),
    ].filter((time) => Date.parse(time) <= Date.parse(NOW));
    const result = prune([...real, ...junk]);
    expect(result.code).toBe(0);
    const forget = resticCommands(result.calls)[1];
    // keep-within keeps every snapshot within the window, counted from the newest one; with no
    // other keep rule, junk cannot take a real snapshot's place.
    expect(keepFlags(forget)).toEqual(["--keep-within", "1y"]);
    const newest = Math.max(...[...real, ...junk].map((time) => Date.parse(time)));
    const yearBefore = new Date(newest);
    yearBefore.setUTCFullYear(yearBefore.getUTCFullYear() - 1);
    expect(real.filter((time) => Date.parse(time) < yearBefore.getTime())).toEqual([]);
  });

  test("VPS-006 accepts two snapshots on one UTC day (a manual run beside the timer)", () => {
    const result = prune([...dailyTimes(5), "2026-11-04T09:00:00.123456789+02:00"]);
    expect(result.code).toBe(0);
    expect(commandNames(result)).toContain("forget");
  });

  const refusals: [string, string[], string][] = [
    ["a snapshot is dated more than an hour in the future", [...dailyTimes(20), "2027-01-01T03:15:00Z"], "in the future"],
    ["a snapshot is dated in the future through its time zone", [...dailyTimes(3), "2026-11-04T08:30:00-05:00"], "in the future"],
    ["a UTC day has three snapshots", [...dailyTimes(20), "2026-11-04T01:00:00Z", "2026-11-04T10:30:00+01:00"], "3 snapshots on one UTC day"],
    ["a snapshot time cannot be parsed", [...dailyTimes(3), "yesterday"], "nothing was pruned"],
  ];
  test.each(refusals)("VPS-006 refuses to prune and never forgets when %s", (_name, times, message) => {
    const result = prune(times);
    expect(result.code).not.toBe(0);
    expect(result.out).toContain(message);
    expect(commandNames(result)).toEqual(["snapshots"]);
  });

  test("VPS-006 refuses to prune when the snapshot list cannot be read", () => {
    const result = prune(dailyTimes(3), { FAKE_FAIL: "restic-snapshots" });
    expect(result.code).not.toBe(0);
    expect(commandNames(result)).toEqual(["snapshots"]);
  });

  test("VPS-006 refuses to prune without jq", () => {
    const result = prune(dailyTimes(3), { JQ: "/nonexistent/jq" });
    expect(result.code).not.toBe(0);
    expect(result.out).toContain("jq is required");
    expect(resticCommands(result.calls)).toEqual([]);
  });

  test("VPS-006 prune exits non-zero when restic forget fails", () => {
    const result = prune(dailyTimes(3), { FAKE_FAIL: "restic-forget" });
    expect(result.code).not.toBe(0);
    expect(result.out).toContain("forget --prune failed");
  });
});
