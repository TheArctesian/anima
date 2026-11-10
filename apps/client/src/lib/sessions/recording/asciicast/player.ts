import type { PlaybackTerminal } from '../../screen/playback';
import type { Cast } from './parse';
import { durationOf, eventsBy } from './timeline';

/** Time for the player: `now` in milliseconds, `after` runs `run` once after `ms` and returns how to cancel it. */
export interface PlaybackClock {
  now(): number;
  after(ms: number, run: () => void): () => void;
}

export interface PlaybackView {
  readonly playing: boolean;
  /** Seconds into the recording. */
  readonly position: number;
  readonly duration: number;
}

/** A recording shown in a terminal with play, pause and seek (RA-041, D-12). */
export interface CastPlayer {
  play(): void;
  pause(): void;
  /** Show the screen as it was at `time` seconds (clamped to the recording) and carry on from there if playing. */
  seek(time: number): void;
  /** Replace the recording with a longer one (the same recording, read further); the position stays. */
  extend(cast: Cast): void;
  view(): PlaybackView;
  /** Called after play, pause, seek, extend and when the end is reached. */
  onchange(listener: () => void): () => void;
}

/** The browser's clock for the player. */
export const browserClock: PlaybackClock = {
  now: () => performance.now(),
  after: (ms, run) => {
    const id = setTimeout(run, ms);
    return () => {
      clearTimeout(id);
    };
  },
};

/** What a player shows before there is a recording to play. */
const NO_SECONDS = 0;
export const NOTHING_TO_PLAY: PlaybackView = { playing: false, position: NO_SECONDS, duration: NO_SECONDS };

/** What has been written to the terminal of the recording so far; it writes forward and rewinds with one reset. */
interface Screen {
  /** Make the terminal show `cast` as it was at `time`. */
  show(cast: Cast, time: number): void;
  /** The number of events of the recording already on the screen. */
  shown(): number;
}

function screenOf(terminal: PlaybackTerminal): Screen {
  let applied = 0;
  /** Write the events from `applied` up to `count`, joining neighbouring output into one write. */
  const writeTo = (cast: Cast, count: number) => {
    let text = '';
    const flush = () => {
      if (text !== '') terminal.write(text);
      text = '';
    };
    for (const event of cast.events.slice(applied, count)) {
      if (event.kind === 'output') {
        text += event.text;
      } else {
        flush();
        terminal.resize(event.cols, event.rows);
      }
    }
    flush();
    applied = count;
  };
  return {
    show(cast, time) {
      const count = eventsBy(cast, time);
      if (count < applied) {
        terminal.reset();
        terminal.resize(cast.header.width, cast.header.height);
        applied = 0;
      }
      writeTo(cast, count);
    },
    shown: () => applied,
  };
}

/** The position in the recording: held while stopped, moving with the clock while running. */
function timekeeper(clock: PlaybackClock, limit: () => number) {
  let held = 0;
  let running = false;
  let startedAt = 0;
  let startedFrom = 0;
  const clamp = (time: number) => Math.min(Math.max(Number.isFinite(time) ? time : 0, 0), limit());
  return {
    clamp,
    after: clock.after,
    running: () => running,
    read: () => (running ? clamp(startedFrom + (clock.now() - startedAt) / 1000) : held),
    hold(time: number) {
      held = time;
      running = false;
    },
    run(from: number) {
      held = from;
      running = true;
      startedAt = clock.now();
      startedFrom = from;
    },
  };
}

type Timekeeper = ReturnType<typeof timekeeper>;

/** Runs the recording from a position: wakes at each next event, shows it, and holds at the end. */
function playbackLoop(cast: () => Cast, screen: Screen, time: Timekeeper, onend: () => void) {
  let cancel: (() => void) | undefined;
  const schedule = () => {
    const next = cast().events[screen.shown()];
    const wait = Math.max(0, ((next?.at ?? durationOf(cast())) - time.read()) * 1000);
    cancel = time.after(wait, tick);
  };
  function tick() {
    const now = time.read();
    screen.show(cast(), now);
    if (now < durationOf(cast())) {
      schedule();
      return;
    }
    time.hold(now);
    onend();
  }
  return {
    runFrom(from: number) {
      time.run(from);
      schedule();
    },
    stop() {
      cancel?.();
      cancel = undefined;
    },
  };
}

export function createCastPlayer(terminal: PlaybackTerminal, first: Cast, clock: PlaybackClock): CastPlayer {
  let cast = first;
  const screen = screenOf(terminal);
  const time = timekeeper(clock, () => durationOf(cast));
  const listeners = new Set<() => void>();
  const announce = () => {
    for (const listener of listeners) listener();
  };
  const loop = playbackLoop(() => cast, screen, time, announce);

  terminal.resize(cast.header.width, cast.header.height);
  screen.show(cast, 0);
  time.hold(0);

  return {
    play() {
      if (time.running()) return;
      if (time.read() >= durationOf(cast)) {
        screen.show(cast, 0);
        time.hold(0);
      }
      loop.runFrom(time.read());
      announce();
    },
    pause() {
      if (!time.running()) return;
      loop.stop();
      time.hold(time.read());
      announce();
    },
    seek(target) {
      const was = time.running();
      loop.stop();
      const at = time.clamp(target);
      screen.show(cast, at);
      time.hold(at);
      if (was && at < durationOf(cast)) loop.runFrom(at);
      announce();
    },
    extend(longer) {
      const was = time.running();
      const at = time.read();
      loop.stop();
      cast = longer;
      time.hold(at);
      if (was) loop.runFrom(at);
      announce();
    },
    view: () => ({ playing: time.running(), position: time.read(), duration: durationOf(cast) }),
    onchange(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
