// Test support: a stand-in for the Guacamole client's `createArgumentValueStream` that records each argument value
// stream as the gateway would read it. The real writer in `tunnel/argv.ts` writes to it. No vitest import here.
import type { ArgumentClient } from './arguments';

/** One `argv` stream: the instruction's mimetype and name, the decoded text of its blobs, and its `end`s. */
export interface ArgumentStream {
  readonly mimetype: string;
  readonly name: string;
  readonly blobs: string[];
  ends: number;
}

/** The UTF-8 text of base64 `blobs`, joined in order. */
function decoded(blobs: readonly string[]): string {
  return Buffer.concat(blobs.map((blob) => Buffer.from(blob, 'base64'))).toString('utf8');
}

export function fakeArgumentClient() {
  const streams: ArgumentStream[] = [];
  const client: ArgumentClient = {
    createArgumentValueStream: (mimetype, name) => {
      const stream: ArgumentStream = { mimetype, name, blobs: [], ends: 0 };
      streams.push(stream);
      const output = {
        index: streams.length,
        onack: null,
        sendBlob: (blob: string) => stream.blobs.push(blob),
        sendEnd: () => {
          stream.ends += 1;
        },
      };
      return output as unknown as ReturnType<ArgumentClient['createArgumentValueStream']>;
    },
  };
  /** Each stream as `{ mimetype, name, text, ends }`, in the order they were opened. */
  const sent = () => streams.map(({ mimetype, name, blobs, ends }) => ({ mimetype, name, text: decoded(blobs), ends }));
  return { client, sent };
}
