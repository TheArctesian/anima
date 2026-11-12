import type { BuilderShell, ToolCheck } from '$lib/desktop/builder/shell';
import type { Builder, BuildsApi } from './api';

export interface BuildsPanelPorts {
  /** With the operator's session (BLD-025). */
  readonly api: Pick<BuildsApi, 'builders' | 'revoke'>;
  readonly shell: Pick<BuilderShell, 'checkTools'>;
  /** Called after builder `id` was revoked, so this computer forgets its token when the ID is its own. */
  readonly revoked: (id: string) => Promise<void>;
}

/** What the Builds row shows besides the toggle: the tools and the list of builders (BLD-020, BLD-025, BLD-028). */
export class BuildsPanel {
  tools = $state<ToolCheck | null>(null);
  toolsFailed = $state(false);
  builders = $state<readonly Builder[]>([]);
  /** Whether the list of builders has been read once, so that an empty list is not shown before it is known. */
  buildersLoaded = $state(false);
  listProblem = $state<string | null>(null);
  revoking = $state<string | null>(null);

  readonly #ports: BuildsPanelPorts;

  constructor(ports: BuildsPanelPorts) {
    this.#ports = ports;
  }

  /** Reads the tools and the builders; each part fails on its own. */
  async load(): Promise<void> {
    await Promise.all([this.#loadTools(), this.loadBuilders()]);
  }

  async #loadTools(): Promise<void> {
    try {
      this.tools = await this.#ports.shell.checkTools();
      this.toolsFailed = false;
    } catch {
      this.toolsFailed = true;
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
