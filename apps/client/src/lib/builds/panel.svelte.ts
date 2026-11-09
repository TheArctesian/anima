import type { BuilderShell, ToolCheck } from '$lib/desktop/builder/shell';
import { UpdaterKeyError, type UpdaterKeyState } from '$lib/desktop/builder/updaterKey';
import type { Builder, BuildsApi } from './api';

/** The updater key calls of the shell (BLD-045). */
export interface UpdaterKeyPort {
  read(): Promise<UpdaterKeyState>;
  import(password: string | null): Promise<UpdaterKeyState>;
}

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

/** What the Builds row shows besides the toggle: the tools, the updater key and the list of builders (BLD-020, BLD-025, BLD-028). */
export class BuildsPanel {
  tools = $state<ToolCheck | null>(null);
  toolsFailed = $state(false);
  key = $state<UpdaterKeyState | null>(null);
  keyProblem = $state<string | null>(null);
  importing = $state(false);
  builders = $state<readonly Builder[]>([]);
  listProblem = $state<string | null>(null);
  revoking = $state<string | null>(null);

  readonly #ports: BuildsPanelPorts;

  constructor(ports: BuildsPanelPorts) {
    this.#ports = ports;
  }

  /** Reads the tools, the key state and the builders; each part fails on its own. */
  async load(): Promise<void> {
    await Promise.all([this.#loadTools(), this.#loadKey(), this.loadBuilders()]);
  }

  async #loadTools(): Promise<void> {
    try {
      this.tools = await this.#ports.shell.checkTools();
      this.toolsFailed = false;
    } catch {
      this.toolsFailed = true;
    }
  }

  async #loadKey(): Promise<void> {
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
  }

  /** Imports the updater key from a file the operator picks in the shell's dialog (BLD-045). */
  async importKey(password: string): Promise<boolean> {
    if (this.importing) return false;
    this.importing = true;
    this.keyProblem = null;
    try {
      this.key = await this.#ports.updaterKey.import(password === '' ? null : password);
      await this.#loadTools();
      return true;
    } catch (reason) {
      this.keyProblem = keyProblem(reason);
      return false;
    } finally {
      this.importing = false;
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
