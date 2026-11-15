import {
  chooseLevel,
  onStayAwake,
  readStayAwake,
  removeLidSetUp,
  setBatteryFloor,
  setUpLid,
  type Level,
  type StayAwakeResult,
  type StayAwakeStatus,
} from '$lib/desktop/stay-awake/shell';
import { problemText } from './stay-awake';

/** The shell calls the rows make; a test passes its own. */
export interface StayAwakeShell {
  readonly read: () => Promise<StayAwakeResult>;
  readonly choose: (level: Level) => Promise<StayAwakeResult>;
  readonly setFloor: (floor: number | null) => Promise<StayAwakeResult>;
  readonly setUpLid: () => Promise<StayAwakeResult>;
  readonly removeLidSetUp: () => Promise<StayAwakeResult>;
  readonly onChange: (listener: (status: StayAwakeStatus) => void) => Promise<() => void>;
}

export const desktopStayAwakeShell: StayAwakeShell = {
  read: readStayAwake,
  choose: chooseLevel,
  setFloor: setBatteryFloor,
  setUpLid,
  removeLidSetUp,
  onChange: onStayAwake,
};

/**
 * The "Stay awake" rows (AWK-004): the state the shell reports and the operator's changes. One call at a time; each
 * answer replaces the state, and so does each change the shell announces (the tray item, the battery).
 */
export class StayAwakeControl {
  status = $state<StayAwakeStatus | null>(null);
  busy = $state(false);
  problem = $state<string | null>(null);

  constructor(private readonly shell: StayAwakeShell) {}

  private async run(call: () => Promise<StayAwakeResult>): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.problem = null;
    const result = await call();
    this.busy = false;
    if (result.ok) this.status = result.status;
    else this.problem = problemText(result.problem);
  }

  /** Reads the state and listens for changes; returns the function that stops listening. */
  watch(): () => void {
    let stopped = false;
    let stop: (() => void) | undefined;
    void this.shell
      .onChange((status) => {
        this.status = status;
      })
      .then((unlisten) => {
        if (stopped) unlisten();
        else stop = unlisten;
      });
    void this.run(() => this.shell.read());
    return () => {
      stopped = true;
      stop?.();
    };
  }

  choose(level: Level): Promise<void> {
    return this.run(() => this.shell.choose(level));
  }

  setFloor(floor: number | null): Promise<void> {
    return this.run(() => this.shell.setFloor(floor));
  }

  setUpLid(): Promise<void> {
    return this.run(() => this.shell.setUpLid());
  }

  removeLidSetUp(): Promise<void> {
    return this.run(() => this.shell.removeLidSetUp());
  }
}
