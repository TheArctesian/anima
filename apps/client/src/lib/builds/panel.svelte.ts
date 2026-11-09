import type { BuilderShell, ToolCheck } from '$lib/desktop/builder/shell';
import { UpdaterKeyError, type UpdaterKeyState } from '$lib/desktop/builder/updater-key';
import type { Builder, BuildsApi } from './api';

/** The updater key calls of the shell (BLD-045, D-19). */
export interface UpdaterKeyPort {
  read(): Promise<UpdaterKeyState>;
  import(password: string | null, newPassword: string | null): Promise<UpdaterKeyState>;
  unlock(password: string): Promise<UpdaterKeyState>;
}

/** The password field a key problem is about, so the row can mark it and move focus to it (UI-013). */
export type KeyField = 'file-password' | 'new-password' | 'unlock-password';

export interface BuildsPanelPorts {
  /** With the operator's session (BLD-025). */
  readonly api: Pick<BuildsApi, 'builders' | 'revoke'>;
  readonly shell: Pick<BuilderShell, 'checkTools'>;
  readonly updaterKey: UpdaterKeyPort;
  /** Called after builder `id` was revoked, so this computer forgets its token when the ID is its own. */
  readonly revoked: (id: string) => Promise<void>;
}

function keyProblem(reason: unknown): string {
  return reason instanceof UpdaterKeyError ? reason.message : new UpdaterKeyError('failed').message;
}

/** The import field that `reason` is about; `null` when it is about no field. */
function importField(reason: unknown): KeyField | null {
  if (!(reason instanceof UpdaterKeyError)) return null;
  if (reason.problem === 'password-needed' || reason.problem === 'password') return 'file-password';
  return reason.problem === 'new-password-needed' ? 'new-password' : null;
}

/** What the Builds row shows besides the toggle: the tools, the updater key and the list of builders (BLD-020, BLD-025, BLD-028). */
export class BuildsPanel {
  tools = $state<ToolCheck | null>(null);
  toolsFailed = $state(false);
  key = $state<UpdaterKeyState | null>(null);
  keyProblem = $state<string | null>(null);
  /** The field `keyProblem` is about. */
  keyField = $state<KeyField | null>(null);
  /** Whether the import asked for a new password: the key file has none (D-19). */
  newPasswordNeeded = $state(false);
  importing = $state(false);
  unlocking = $state(false);
  builders = $state<readonly Builder[]>([]);
  /** Whether the list of builders has been read once, so that an empty list is not shown before it is known. */
  buildersLoaded = $state(false);
  listProblem = $state<string | null>(null);
  revoking = $state<string | null>(null);

  readonly #ports: BuildsPanelPorts;

  constructor(ports: BuildsPanelPorts) {
    this.#ports = ports;
  }

  /** Reads the tools, the key state and the builders; each part fails on its own. */
  async load(): Promise<void> {
    await Promise.all([this.#loadTools(), this.loadKey(), this.loadBuilders()]);
  }

  async #loadTools(): Promise<void> {
    try {
      this.tools = await this.#ports.shell.checkTools();
      this.toolsFailed = false;
    } catch {
      this.toolsFailed = true;
    }
  }

  /** Reads the key state again, for example after turning building off locked the key. */
  async loadKey(): Promise<void> {
    try {
      this.key = await this.#ports.updaterKey.read();
    } catch (reason) {
      this.keyProblem = keyProblem(reason);
    }
  }

  async loadBuilders(): Promise<void> {
    const result = await this.#ports.api.builders();
    if (result.ok) {
      this.builders = result.value;
      this.listProblem = null;
    } else {
      this.listProblem = 'The list of builders could not be read.';
    }
    this.buildersLoaded = true;
  }

  #failed(reason: unknown, field: KeyField | null): void {
    this.keyProblem = keyProblem(reason);
    this.keyField = field;
  }

  /**
   * Imports the updater key from a file the operator picks in the shell's dialog (BLD-045). A file without a password is
   * stored sealed with `newPassword` (D-19).
   */
  async importKey(password: string, newPassword: string): Promise<boolean> {
    if (this.importing || this.unlocking) return false;
    this.importing = true;
    this.keyProblem = null;
    this.keyField = null;
    try {
      this.key = await this.#ports.updaterKey.import(password === '' ? null : password, newPassword === '' ? null : newPassword);
      this.newPasswordNeeded = false;
      await this.#loadTools();
      return true;
    } catch (reason) {
      const field = importField(reason);
      if (field === 'new-password') this.newPasswordNeeded = true;
      this.#failed(reason, field);
      return false;
    } finally {
      this.importing = false;
    }
  }

  /** Opens the stored updater key with `password` for this app session (D-19). */
  async unlockKey(password: string): Promise<boolean> {
    if (this.importing || this.unlocking) return false;
    this.unlocking = true;
    this.keyProblem = null;
    this.keyField = null;
    try {
      this.key = await this.#ports.updaterKey.unlock(password);
      await this.#loadTools();
      return true;
    } catch (reason) {
      const wrong = reason instanceof UpdaterKeyError && reason.problem === 'password';
      this.#failed(reason, wrong ? 'unlock-password' : null);
      return false;
    } finally {
      this.unlocking = false;
    }
  }

  /** Revokes builder `id` on the hub (BLD-025). */
  async revoke(id: string): Promise<void> {
    if (this.revoking !== null) return;
    this.revoking = id;
    try {
      const result = await this.#ports.api.revoke(id);
      if (result.ok || result.error.code === 'NOT_FOUND') {
        await this.#ports.revoked(id);
        await this.loadBuilders();
      } else {
        this.listProblem = 'The hub could not revoke that builder.';
      }
    } finally {
      this.revoking = null;
    }
  }
}
