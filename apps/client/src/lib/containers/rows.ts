import type { Container, LogLine } from './api';

/** The group of the containers that no Compose project owns; it is drawn last. */
export const NO_PROJECT = 'No Compose project';

/** The numbers of log lines the operator can choose; the hub accepts 1 to 1000 (HOST-001). */
export const LINE_CHOICES = ['100', '200', '500', '1000'] as const;
export type LineChoice = (typeof LINE_CHOICES)[number];
export const DEFAULT_LINES: LineChoice = '200';

export type Tone = 'up' | 'down' | 'warning' | 'unknown';

export interface ContainerRow {
  readonly id: string;
  /** The first 12 digits of the identifier, as Docker shows it. */
  readonly shortId: string;
  readonly name: string;
  readonly image: string;
  /** The state as a word: "Running", "Exited". */
  readonly state: string;
  readonly tone: Tone;
  /** Docker's status text: "Up 3 hours (healthy)". */
  readonly status: string;
  readonly ports: string;
  readonly service: string;
}

export interface ProjectGroup {
  /** The Compose project, or `NO_PROJECT`. */
  readonly project: string;
  /** "2 of 3 running". */
  readonly summary: string;
  readonly rows: readonly ContainerRow[];
}

const STATE_WORDS: Readonly<Record<Container['state'], string>> = {
  RUNNING: 'Running',
  EXITED: 'Exited',
  RESTARTING: 'Restarting',
  PAUSED: 'Paused',
  CREATED: 'Created',
  REMOVING: 'Removing',
  DEAD: 'Dead',
  UNKNOWN: 'Unknown',
};

/** The colour of a state is never its only sign: the word is always beside it (MAT-005). */
function tone(container: Container): Tone {
  if (container.state === 'RUNNING') return container.health === 'UNHEALTHY' ? 'down' : container.health === 'STARTING' ? 'warning' : 'up';
  if (container.state === 'DEAD') return 'down';
  if (container.state === 'RESTARTING' || container.state === 'PAUSED' || container.state === 'REMOVING') return 'warning';
  return 'unknown';
}

function stateWord(container: Container): string {
  if (container.state !== 'RUNNING') return STATE_WORDS[container.state];
  if (container.health === 'UNHEALTHY') return 'Running, unhealthy';
  if (container.health === 'STARTING') return 'Running, starting';
  return 'Running';
}

function row(container: Container): ContainerRow {
  return {
    id: container.id,
    shortId: container.id.slice(0, 12),
    name: container.name,
    image: container.image,
    state: stateWord(container),
    tone: tone(container),
    status: container.status,
    ports: container.ports.join(', '),
    service: container.composeService ?? '',
  };
}

/** HOST-013: the containers in groups by Compose project, the projects by name, then the containers with none; in a group by name. */
export function projectGroups(containers: readonly Container[]): readonly ProjectGroup[] {
  const groups = new Map<string, Container[]>();
  for (const container of containers) {
    const project = container.composeProject ?? NO_PROJECT;
    groups.set(project, [...(groups.get(project) ?? []), container]);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => Number(a === NO_PROJECT) - Number(b === NO_PROJECT) || a.localeCompare(b))
    .map(([project, members]) => ({
      project,
      summary: `${String(members.filter((member) => member.state === 'RUNNING').length)} of ${String(members.length)} running`,
      rows: members.map(row).sort((a, b) => a.name.localeCompare(b.name)),
    }));
}

/** The text of a log, one line for each line of the container, with no time: what the operator copies. */
export function logText(lines: readonly LogLine[]): string {
  return lines.map((line) => line.text).join('\n');
}

/** The time of a log line as the operator's clock shows it, `18:00:04`; empty when the line has no time. */
export function lineTime(at: string | null | undefined): string {
  if (at == null) return '';
  const time = new Date(at);
  return Number.isNaN(time.getTime()) ? '' : time.toLocaleTimeString([], { hour12: false });
}
