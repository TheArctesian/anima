import type { IconName } from '$lib/icons/icons';
import type { ChecksState, CommitChecks, MergeState, RepositoryAccess, WorkflowRun } from './types';

/** The first seven digits, as GitHub shows a commit. */
export const shortSha = (sha: string): string => sha.slice(0, 7);

/** Why the lists of a repository cannot be read, and what to do; null when they can (PRJ-032). */
export function accessWords(access: RepositoryAccess): { title: string; line: string } | null {
  switch (access) {
    case 'READY':
      return null;
    case 'NOT_CONNECTED':
      return { title: 'Not connected', line: 'No GitHub account holds this repository. Connect GitHub in Settings, Integrations.' };
    case 'NEEDS_RECONNECT':
      return { title: 'GitHub refused the key', line: 'Reconnect the account that holds this repository in Settings, Integrations.' };
    case 'PAUSED':
      return { title: 'The account is paused', line: 'Resume the account that holds this repository in Settings, Integrations.' };
    case 'LEFT_OUT':
      return { title: 'Left out', line: 'This repository is left out of Anima. Restore it on the Overview tab to read it.' };
    case 'GONE':
      return { title: 'No longer on GitHub', line: 'The connection no longer lists this repository.' };
  }
}

/** A mark and its words for the checks of a commit; the words go with the mark, so colour is never alone (UI-003). */
export function checksMark(checks: CommitChecks | null | undefined): { icon: IconName; tone: string; words: string } {
  const state: ChecksState = checks?.state ?? 'NONE';
  switch (state) {
    case 'PASSED':
      return { icon: 'carbon:checkmark-filled', tone: 'up', words: 'Checks passed' };
    case 'FAILED':
      return { icon: 'carbon:close-filled', tone: 'down', words: `${String(checks?.failed ?? 0)} failed` };
    case 'PENDING':
      return { icon: 'carbon:circle-dash', tone: 'running', words: 'Checks running' };
    case 'NONE':
      return { icon: 'carbon:circle-dash', tone: 'quiet', words: 'No checks' };
  }
}

export const MERGE_STATE_WORDS: Readonly<Record<MergeState, string>> = {
  CLEAN: 'Ready to merge',
  UNSTABLE: 'A check that is not required failed',
  CONFLICT: 'Has conflicts',
  BLOCKED: 'A required check or review is missing',
  BEHIND: 'Behind its base branch',
  DRAFT: 'Draft',
  UNKNOWN: 'GitHub is still working it out',
};

/** A run's state in words and the tone of its mark. */
export function runState(run: WorkflowRun): { words: string; tone: string; icon: IconName } {
  if (run.status === 'QUEUED') return { words: 'Queued', tone: 'running', icon: 'carbon:circle-dash' };
  if (run.status === 'IN_PROGRESS') return { words: 'Running', tone: 'running', icon: 'carbon:circle-dash' };
  switch (run.conclusion) {
    case 'SUCCESS':
      return { words: 'Succeeded', tone: 'up', icon: 'carbon:checkmark-filled' };
    case 'FAILURE':
      return { words: 'Failed', tone: 'down', icon: 'carbon:close-filled' };
    case 'TIMED_OUT':
      return { words: 'Timed out', tone: 'down', icon: 'carbon:close-filled' };
    case 'CANCELLED':
      return { words: 'Cancelled', tone: 'quiet', icon: 'carbon:subtract-alt' };
    case 'ACTION_REQUIRED':
      return { words: 'Needs approval on GitHub', tone: 'warning', icon: 'carbon:warning-alt' };
    case 'SKIPPED':
      return { words: 'Skipped', tone: 'quiet', icon: 'carbon:subtract-alt' };
    default:
      return { words: 'Finished', tone: 'quiet', icon: 'carbon:subtract-alt' };
  }
}

/** Whether a run is still going, so the runs are read again every 30 seconds (PRJ-034). */
export const running = (run: WorkflowRun): boolean => run.status !== 'COMPLETED';
