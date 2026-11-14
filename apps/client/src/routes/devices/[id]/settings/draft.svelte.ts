import { FORM_FIELD, fieldErrorsFrom } from '$lib/devices/form/field-errors';
import type { EditedValues } from '$lib/devices/form/form/editing';
import { validateDevice } from '$lib/devices/form/validate';
import type { DeviceFormValues } from '$lib/devices/form/values';
import type { FieldError, HubResult } from '$lib/hub/hub';
import { duration } from '$lib/motion/tokens';
import { groupOf, inGroup, replaceGroup, sameGroup, setValueAt, valueAt, withGroup, type SettingKey, type SettingsGroup } from './groups';

/** What the mark beside a setting's label says: "Saving…", "Saved" (which then fades), or that the value was refused. */
export type SaveMark = 'saving' | 'saved' | 'faded' | 'refused';

/** Runs `run` after `milliseconds`; answers the function that cancels it. */
export type Later = (run: () => void, milliseconds: number) => () => void;

/** The secrets one Save password sends: the SSH credential, or an RDP or VNC password. */
export type SecretValues = Pick<DeviceFormValues, 'credential' | 'remoteSecrets'>;

interface Waiting {
  work: () => Promise<boolean>;
  readonly done: Promise<void>;
}

function browserLater(run: () => void, milliseconds: number): () => void {
  const timer = setTimeout(run, milliseconds);
  return () => {
    clearTimeout(timer);
  };
}

/**
 * The device settings as shown, saved as each change is made (UR-7, D17). A change validates at once and then waits its
 * turn: saves go one at a time, each built on the device as last saved, and a change made again while its earlier save
 * still waits replaces that save. A refused change stays shown, unsaved, with its reasons in `errors`.
 */
export class SettingsDraft {
  /** The settings as shown; a deep state, so each field's edit is seen. */
  values = $state<EditedValues>({} as EditedValues);
  marks = $state<Partial<Record<string, SaveMark>>>({});
  errors = $state.raw<readonly FieldError[]>([]);
  readonly #saved: () => DeviceFormValues;
  readonly #save: (values: DeviceFormValues) => Promise<HubResult<unknown>>;
  readonly #later: Later;
  #queue: Promise<void> = Promise.resolve();
  #waiting: Partial<Record<string, Waiting>> = {};
  /** Saves of each setting that are waiting or on their way. */
  #open: Partial<Record<string, number>> = {};
  #fades: Partial<Record<string, () => void>> = {};

  /** `saved` reads the device as the hub last stored it; `save` is the whole-device write (`saveDevice`). */
  constructor(saved: () => DeviceFormValues, save: (values: DeviceFormValues) => Promise<HubResult<unknown>>, later: Later = browserLater) {
    this.#saved = saved;
    this.#save = save;
    this.#later = later;
    this.values = structuredClone(saved()) as EditedValues;
  }

  /** A copy of the settings as shown now, which later edits cannot reach. They are text, flags and lists only. */
  #shown(): DeviceFormValues {
    return JSON.parse(JSON.stringify(this.values)) as DeviceFormValues;
  }

  /** The errors of one field path, for its outline and the line under it. */
  errorOf(field: string): FieldError | undefined {
    return this.errors.find((error) => error.field === field);
  }

  /** The errors of a group, for the one line under its fields. */
  errorsIn(group: SettingsGroup): FieldError[] {
    return this.errors.filter((error) => inGroup(error.field, group));
  }

  /**
   * Saves the setting at `key` as shown, with the rest of its group (D17): a switch, a choice, a step or a tag at once,
   * a text field when it is left or Return is pressed. A value that does not validate is refused here and never sent.
   */
  commit(key: SettingKey): Promise<void> {
    const group = groupOf(key);
    const shown = this.#shown();
    if ((this.#open[key] ?? 0) === 0 && sameGroup(shown, this.#saved(), group)) {
      this.#forget(key, group);
      return Promise.resolve();
    }
    const refused = validateDevice(withGroup(this.#saved(), shown, group)).filter((error) => inGroup(error.field, group));
    if (refused.length > 0) {
      this.errors = replaceGroup(this.errors, group, refused);
      this.refuse(key);
      return Promise.resolve();
    }
    return this.run(key, async () => {
      const result = await this.#save(withGroup(this.#saved(), shown, group));
      if (!result.ok) {
        const found = fieldErrorsFrom(result.error).map((error) => (error.field === FORM_FIELD ? { ...error, field: key } : error));
        this.errors = replaceGroup(this.errors, group, found);
        return false;
      }
      this.errors = replaceGroup(this.errors, group, []);
      this.#settleGroup(group, shown);
      return true;
    });
  }

  /**
   * Stores a typed secret on its own (D17: a secret is saved on purpose, by Save password). Answers null once stored, or
   * the sentence that says why not; the caller drops the typed secret either way it is done with it.
   */
  async saveSecret(key: SettingKey, secrets: SecretValues): Promise<string | null> {
    const sent: DeviceFormValues = { ...this.#saved(), ...secrets };
    const invalid = validateDevice(sent).find((error) => error.field.startsWith('credential.'));
    if (invalid !== undefined) {
      this.refuse(key);
      return invalid.message;
    }
    let problem: string | null = null;
    await this.run(key, async () => {
      const result = await this.#save({ ...this.#saved(), ...secrets });
      problem = result.ok ? null : result.error.message;
      return result.ok;
    });
    return problem;
  }

  /** Runs one save in its turn, with the mark beside `key`; `work` answers whether the hub took it. */
  run(key: string, work: () => Promise<boolean>): Promise<void> {
    this.#mark(key, 'saving');
    const waiting = this.#waiting[key];
    if (waiting !== undefined) {
      waiting.work = work;
      return waiting.done;
    }
    this.#open[key] = (this.#open[key] ?? 0) + 1;
    const entry: Waiting = {
      work,
      done: this.#queue.then(async () => {
        this.#waiting[key] = undefined;
        try {
          const saved = await entry.work();
          this.#mark(key, saved ? 'saved' : 'refused');
        } finally {
          this.#open[key] = (this.#open[key] ?? 1) - 1;
        }
      }),
    };
    this.#waiting[key] = entry;
    // A save that threw still lets the next one run; the caller of the one that threw hears it.
    this.#queue = entry.done.catch(() => undefined);
    return entry.done;
  }

  /** `key` is shown as saved again: a "Not saved" beside it goes. */
  unmark(key: string): void {
    if (this.marks[key] === 'refused') this.marks[key] = undefined;
  }

  /** Marks `key` as not saved; its reasons are in `errors` or in its own line. */
  refuse(key: string): void {
    this.#mark(key, 'refused');
  }

  /** Validates the address and one protocol before a connection test; answers whether the test may run. */
  checkForTest(group: 'ssh' | 'rdp' | 'vnc'): boolean {
    const found = validateDevice(this.#shown()).filter(
      (error) => error.field === 'host' || inGroup(error.field, group),
    );
    this.errors = [...this.errors.filter((error) => error.field !== 'host' && !inGroup(error.field, group)), ...found];
    return found.length === 0;
  }

  /** Each field of the group still shown as it was sent takes the saved form, such as a trimmed name or a bare host. */
  #settleGroup(group: SettingsGroup, sent: DeviceFormValues): void {
    const saved = this.#saved();
    const part = sent[group];
    const keys: SettingKey[] =
      typeof part === 'object' && !Array.isArray(part) ? Object.keys(part).map((field) => `${group}.${field}` as SettingKey) : [group];
    const shown = this.#shown();
    for (const key of keys) {
      if (JSON.stringify(valueAt(shown, key)) === JSON.stringify(valueAt(sent, key))) setValueAt(this.values, key, valueAt(saved, key));
      if (key !== group && this.marks[key] === 'refused') this.marks[key] = undefined;
    }
  }

  /** The group is shown as saved again: its reasons and a refusal go. */
  #forget(key: string, group: SettingsGroup): void {
    this.errors = replaceGroup(this.errors, group, []);
    this.unmark(key);
  }

  #mark(key: string, mark: SaveMark): void {
    this.#fades[key]?.();
    this.#fades[key] = undefined;
    this.marks[key] = mark;
    if (mark !== 'saved') return;
    this.#fades[key] = this.#later(() => {
      this.#fades[key] = undefined;
      if (this.marks[key] === 'saved') this.marks[key] = 'faded';
    }, duration.highlight);
  }
}
