/** The build-time values this module reads. `trial-bundle.sh` sets the one below; a release build sets none. */
interface BuildValues {
  readonly VITE_ANIMA_LOCAL_BUILD?: string;
}

/** What the Settings row says in a local build, in place of the update state (UPD-006, DEF-001). */
export const LOCAL_BUILD_LINE = 'This is a local build. It does not update itself.';

/**
 * Whether this client was built on a computer for a trial (`trial-bundle.sh`), not by a release. Such a build never
 * checks the feed: the published version can be lower than its own, and the updater would install it (UPD-001, DEF-001).
 */
export function isLocalBuild(values: BuildValues = import.meta.env as BuildValues): boolean {
  return values.VITE_ANIMA_LOCAL_BUILD === '1';
}
