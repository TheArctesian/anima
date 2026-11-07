const NOT_STOPPED = 'The session could not be stopped.';

/** Which sessions the operator is stopping now, and which Stop the hub refused (RA-034). */
export class Stopping {
  #busy = $state<readonly string[]>([]);
  #refused = $state<readonly string[]>([]);

  isStopping = (id: string): boolean => this.#busy.includes(id);

  /** Fixed text for a refused Stop of `id`, else null. */
  failure = (id: string): string | null => (this.#refused.includes(id) ? NOT_STOPPED : null);

  /** Mark `id` busy and clear its failure; false when it was busy already. */
  begin(id: string): boolean {
    if (this.#busy.includes(id)) return false;
    this.#busy = [...this.#busy, id];
    this.#refused = this.#refused.filter((failed) => failed !== id);
    return true;
  }

  /** The hub answered: `id` is no longer busy, and failed unless `ok`. */
  end(id: string, ok: boolean): void {
    this.#busy = this.#busy.filter((busy) => busy !== id);
    if (!ok) this.#refused = [...this.#refused, id];
  }
}
