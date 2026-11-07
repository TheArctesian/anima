import type { FilesApi } from './api';
import { endText } from './link/closing';
import type { FileEntry, Listing } from './link/codec';
import type { FilesResult } from './link/inflight';
import { openFilesLink, type FilesLink } from './link/link';
import type { UploadSource } from './link/upload';
import { joinPath, parentPath, saveName, sortedEntries } from './paths';
import { NAME_REFUSED, TICKET_FAILED, problemText } from './problems';
import { saveBlob, type BlobSaver } from './save';
import type { SocketConstructor } from '../tunnel/state';

/** What the panel needs from outside; the tests pass fakes (CQ-021). */
export interface FilesPanelDeps {
  readonly api: FilesApi;
  readonly sessionId: string;
  /** `hub.address.origin`. */
  readonly origin: string;
  readonly WebSocket?: SocketConstructor;
  readonly save?: BlobSaver;
}

/** The transfer under way: its direction, the name shown for it and the bytes moved. */
export interface TransferView {
  readonly direction: 'download' | 'upload';
  readonly name: string;
  readonly done: number;
  /** `undefined` until the hub announces a download's size. */
  readonly size: number | undefined;
}

export type PanelPhase = 'opening' | 'ready' | 'ended';

/** The file panel of one SSH session: its link, the directory shown and the one transfer (RA-032, D-5). */
export class FilesPanelState {
  phase: PanelPhase = $state('opening');
  listing: Listing | undefined = $state.raw();
  transfer: TransferView | undefined = $state.raw();
  /** One fixed sentence: the last failure, or the last transfer's outcome. */
  status = $state('');
  /** A list or transfer is in flight, or the link is not ready: no other action may start. */
  busy = $state(false);
  /** A directory listing is on its way; the panel shows placeholder rows once it takes long (5.7.3, Loading). */
  listLoading = $state(false);

  readonly #deps: FilesPanelDeps;
  #link: FilesLink | undefined;
  #closed = false;

  constructor(deps: FilesPanelDeps) {
    this.#deps = deps;
  }

  /** The link is open and nothing is in flight: the only time an action may start. */
  get idle(): boolean {
    return this.phase === 'ready' && !this.busy;
  }

  /** The entries to show: directories first, each group by name. */
  get entries(): FileEntry[] {
    return this.listing === undefined ? [] : sortedEntries(this.listing.entries);
  }

  /** Ask for the ticket, open the link and list the home directory. */
  async open(): Promise<void> {
    const ticket = await this.#deps.api.openFiles(this.#deps.sessionId);
    if (this.#closed) return;
    if (!ticket.ok) {
      this.#end(TICKET_FAILED);
      return;
    }
    const opening = openFilesLink({
      origin: this.#deps.origin,
      ticket: ticket.value,
      WebSocket: this.#deps.WebSocket,
      onchange: (state) => {
        if (state.phase === 'connected' && this.phase === 'opening') {
          this.phase = 'ready';
          void this.#list('.');
        }
      },
      onend: (state) => {
        if (!this.#closed) this.#end(endText(state.end ?? 'ended'));
      },
    });
    if (!opening.ok) {
      this.#end(endText('not-opened'));
      return;
    }
    this.#link = opening.value;
  }

  /** Close the link; nothing is shown after it. Safe to call more than once. */
  close(): void {
    this.#closed = true;
    this.#link?.close();
  }

  into(entry: FileEntry): void {
    if (!this.idle) return;
    const path = this.listing === undefined ? undefined : joinPath(this.listing.path, entry.name);
    if (path === undefined) this.status = NAME_REFUSED;
    else void this.#list(path);
  }

  up(): void {
    if (this.listing !== undefined) void this.#list(parentPath(this.listing.path));
  }

  refresh(): void {
    if (this.listing !== undefined) void this.#list(this.listing.path);
  }

  async download(entry: FileEntry): Promise<void> {
    if (!this.idle) return;
    const path = this.listing === undefined ? undefined : joinPath(this.listing.path, entry.name);
    if (path === undefined || this.#link === undefined) {
      this.status = NAME_REFUSED;
      return;
    }
    const link = this.#link;
    const result = await this.#transfer('download', entry.name, undefined, (progress) => link.download(path, progress));
    if (result?.ok === true) {
      (this.#deps.save ?? saveBlob)(saveName(entry.name), result.value);
      this.status = 'Download finished.';
    }
  }

  async upload(file: UploadSource & { readonly name: string }): Promise<void> {
    if (!this.idle) return;
    const directory = this.listing?.path;
    const path = directory === undefined ? undefined : joinPath(directory, file.name);
    if (directory === undefined || path === undefined || this.#link === undefined) {
      this.status = NAME_REFUSED;
      return;
    }
    const link = this.#link;
    const result = await this.#transfer('upload', file.name, file.size, (progress) => link.upload(path, file, progress));
    if (result?.ok === true) this.status = 'Upload finished.';
    if (result !== undefined && this.phase === 'ready') await this.#list(directory, true);
  }

  /** Stop the transfer in flight; its result settles once the hub confirms. */
  cancel(): void {
    if (this.transfer !== undefined) this.#link?.cancel();
  }

  async #transfer<T>(
    direction: TransferView['direction'],
    name: string,
    size: number | undefined,
    run: (progress: (done: number, size: number) => void) => Promise<FilesResult<T>>,
  ): Promise<FilesResult<T> | undefined> {
    if (this.busy || this.phase !== 'ready') return undefined;
    this.busy = true;
    this.status = '';
    this.transfer = { direction, name, done: 0, size };
    const result = await run((done, size) => {
      this.transfer = { direction, name, done, size };
    });
    this.transfer = undefined;
    this.busy = false;
    if (!result.ok && this.phase === 'ready') this.status = problemText(result.error);
    return result;
  }

  /** List `path`; `keepStatus` leaves the last transfer's outcome on the status line. */
  async #list(path: string, keepStatus = false): Promise<void> {
    if (this.#link === undefined || this.busy || this.phase !== 'ready') return;
    this.busy = true;
    this.listLoading = true;
    const result = await this.#link.list(path);
    this.busy = false;
    this.listLoading = false;
    if (result.ok) {
      this.listing = result.value;
      if (!keepStatus) this.status = '';
    } else if (this.phase === 'ready') {
      this.status = problemText(result.error);
    }
  }

  #end(text: string): void {
    this.phase = 'ended';
    this.status = text;
    this.busy = false;
    this.listLoading = false;
    this.transfer = undefined;
  }
}
