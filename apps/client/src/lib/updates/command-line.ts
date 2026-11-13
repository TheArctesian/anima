import { problemText, type CommandLineProblem, type CommandLineStatus, type Occupant } from '$lib/desktop/command-line/shell';

/** The words of the row's state (INS-006): always words, never a colour alone. */
export function describeCommandLine(status: CommandLineStatus | null): string {
  if (status === null) return 'Checking…';
  switch (status.state) {
    case 'unavailable':
      return problemText(status.reason);
    case 'not-installed':
      return status.create
        ? `Not installed. Install makes the folder ${status.folder} and puts anima at ${status.path}.`
        : `Not installed. Install puts anima at ${status.path}.`;
    case 'installed':
      return `Installed at ${status.path}.`;
    case 'elsewhere':
      return `${status.path} points at ${status.pointsAt}, not this app. Install points it at this app.`;
  }
}

/** The line to add to the shell profile when the proposed folder is not on the PATH; Anima never edits the profile. */
export function profileAdvice(status: CommandLineStatus | null): { readonly folder: string; readonly line: string } | null {
  if (status?.state !== 'not-installed' || status.profileLine === null) return null;
  return { folder: status.folder, line: status.profileLine };
}

/** Where Install writes and asks the operator to confirm, or `null` when Install has nothing to do. */
export function installTarget(status: CommandLineStatus | null): { readonly folder: string; readonly path: string } | null {
  if (status?.state === 'not-installed' || status?.state === 'elsewhere') return { folder: status.folder, path: status.path };
  return null;
}

/** Whether there is a link or launcher an Anima app made for Remove to delete. */
export function canRemove(status: CommandLineStatus | null): boolean {
  return status?.state === 'installed' || status?.state === 'elsewhere';
}

const OCCUPANTS: Readonly<Record<Occupant['kind'], string>> = {
  file: 'a file',
  folder: 'a folder',
  link: 'a link',
  other: 'a special file',
};

/** The refusal in words, with what is in the way at `path` when the shell said. */
export function refusalText(problem: CommandLineProblem, occupant: Occupant | null, path: string | null): string {
  if (problem !== 'occupied' || occupant === null || path === null) return problemText(problem);
  const to = occupant.linkTarget === null ? '' : ` to ${occupant.linkTarget}`;
  return `${problemText(problem)} There is ${OCCUPANTS[occupant.kind]} at ${path}${to}. Move or delete it yourself, then install.`;
}
