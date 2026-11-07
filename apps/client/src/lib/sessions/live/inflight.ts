import type { LiveSession, SessionChange } from './api';
import { applyChange, withoutSession } from './list';

/** What happened while the newest list query was in flight: its answer is older than these (D-6). */
export class InFlight {
  #sequence = 0;
  #pushed: SessionChange[] = [];
  #stopped: string[] = [];

  /** A new query begins: earlier queries are superseded and what was recorded is dropped. Returns the query's number. */
  begin(): number {
    this.#sequence += 1;
    this.#pushed = [];
    this.#stopped = [];
    return this.#sequence;
  }

  /** Whether query `number` is the newest one asked. */
  isNewest(number: number): boolean {
    return number === this.#sequence;
  }

  pushed(change: SessionChange): void {
    this.#pushed.push(change);
  }

  stopped(id: string): void {
    this.#stopped.push(id);
  }

  /** The query's answer with the changes pushed and the sessions stopped since it was asked. */
  catchUp(answer: readonly LiveSession[]): readonly LiveSession[] {
    return this.#stopped.reduce(withoutSession, this.#pushed.reduce(applyChange, answer));
  }
}
