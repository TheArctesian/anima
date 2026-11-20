// How a job of release.yml that comes after the gates states its condition. GitHub skips a
// job when a job before it in the chain was skipped, and the Linux gates are skipped when the
// same content passed them on `dev`. So such a job does not use the default condition: it
// names the result of each job that it waits for, and then its own condition.

/** The start of the condition of a job that waits for `needs`: none failed, and each passed. */
export function eachPassed(needs: readonly string[]): string {
  return ["!cancelled()", "!failure()", ...needs.map((name) => `needs.${name}.result == 'success'`)].join(" && ");
}

/**
 * The condition of the job after the results of the jobs that it waits for: `""` when it has
 * none of its own. Throws when the condition does not start with `eachPassed(needs)`.
 */
export function ownCondition(condition: string | undefined, needs: readonly string[]): string {
  const start = `\${{ ${eachPassed(needs)}`;
  if (condition === undefined || !condition.startsWith(start) || !condition.endsWith(" }}")) {
    throw new Error(`the condition does not name the result of each job that it waits for: ${condition ?? "none"}`);
  }
  const rest = condition.slice(start.length, -" }}".length);
  if (rest === "") return "";
  if (!rest.startsWith(" && ")) throw new Error(`the condition is not joined with &&: ${condition}`);
  return rest.slice(" && ".length);
}
