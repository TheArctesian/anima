/**
 * The broker process on the VPS (R-055, VP-8, BD-7): systemd starts one for each connection
 * to /run/anima-broker/broker.sock (`Accept=yes`, `MaxConnections=1`). With no argument it
 * serves one request from stdin to stdout. `count-nodes` prints the node count and `check`
 * asks the running socket for the node list; the install (`harden.sh broker`) uses both.
 */

import { connect } from 'node:net';
import { MAX_REQUEST_BYTES, type Reply } from './protocol.ts';
import { tailnetOf } from './nodes.ts';
import { serve } from './serve.ts';
import type { BrokerFiles } from './state.ts';
import { LIST_STEPS, type Step } from './verbs.ts';

const SOCKET = '/run/anima-broker/broker.sock';
const FILES: BrokerFiles = {
  config: '/etc/anima/broker.conf',
  policy: '/opt/anima/headscale/config/policy.hujson',
  state: '/var/lib/anima-broker/state.json',
};
// BD-8: the Headscale socket is on the container's tmpfs, so the CLI runs inside the container.
const HEADSCALE = ['/usr/bin/docker', 'compose', '-f', '/opt/anima/headscale/compose.yaml', 'exec', '-T', 'headscale', 'headscale'];
const STEP_ENV = { PATH: '/usr/bin:/bin', DOCKER_CONFIG: '/var/lib/anima-broker/docker' };
const STEP_MS = 8_000;
const READ_MS = 5_000;
const REQUEST_MS = 45_000;
const CHECK_MS = 55_000;
const MAX_OUTPUT_BYTES = 1_048_576;
const USAGE = 64;

export class StepError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'StepError';
  }
}

/** Reads a stream up to `limit` bytes; null when it is longer. */
async function capped(stream: ReadableStream<Uint8Array>, limit: number): Promise<string | null> {
  const chunks: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of stream) {
    size += chunk.length;
    if (size > limit) return null;
    chunks.push(chunk);
  }
  return new TextDecoder().decode(Buffer.concat(chunks));
}

/**
 * Runs `headscale <step>` in the Headscale container. The argv never holds a key; stderr is
 * dropped because it can echo the arguments (an auth ID).
 */
export async function headscale(step: Step): Promise<string> {
  const child = Bun.spawn([...HEADSCALE, ...step], { stdin: 'ignore', stdout: 'pipe', stderr: 'ignore', env: STEP_ENV, timeout: STEP_MS, killSignal: 'SIGKILL' });
  const output = await capped(child.stdout, MAX_OUTPUT_BYTES);
  if (output === null) child.kill('SIGKILL');
  const code = await child.exited;
  if (code !== 0 || output === null) throw new StepError(`a Headscale step failed (exit ${String(code)})`);
  return output;
}

/** The first line of the stream, at most `limit` bytes are read; null on EOF or time-out with nothing read. */
export async function readLine(stream: ReadableStream<Uint8Array>, limit: number, ms: number): Promise<string | null> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  let late = false;
  const timer = setTimeout(() => {
    late = true;
    void reader.cancel();
  }, ms);
  try {
    while (size <= limit) {
      const { done, value } = await reader.read();
      if (done) break;
      const end = value.indexOf(0x0a);
      chunks.push(end === -1 ? value : value.subarray(0, end));
      size += end === -1 ? value.length : end;
      if (end !== -1) return new TextDecoder().decode(Buffer.concat(chunks));
    }
  } finally {
    clearTimeout(timer);
  }
  return late || chunks.length === 0 ? null : new TextDecoder().decode(Buffer.concat(chunks));
}

async function serveOnce(): Promise<number> {
  const line = await readLine(Bun.stdin.stream(), MAX_REQUEST_BYTES + 1, READ_MS);
  if (line === null) {
    process.stderr.write('anima-broker verb=- outcome=no-request node=- fixes=0\n');
    return 0;
  }
  const deadline = setTimeout(() => {
    process.stderr.write('anima-broker verb=- outcome=deadline node=- fixes=-\n');
    process.exit(1);
  }, REQUEST_MS);
  const result = await serve(line, headscale, FILES, new Date());
  clearTimeout(deadline);
  await Bun.write(Bun.stdout, `${JSON.stringify(result.reply)}\n`);
  process.stderr.write(`${result.log}\n`);
  return 0;
}

async function countNodes(): Promise<number> {
  const [list] = LIST_STEPS;
  if (list === undefined) return 1;
  const tailnet = tailnetOf(JSON.parse(await headscale(list)), []);
  process.stdout.write(`${String(tailnet.nodes.length)}\n`);
  return 0;
}

function ask(path: string, line: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = connect(path);
    let text = '';
    socket.setEncoding('utf8');
    socket.setTimeout(CHECK_MS, () => {
      socket.destroy(new StepError('the broker did not answer in time'));
    });
    socket.on('connect', () => socket.write(line));
    socket.on('data', (chunk: string) => {
      text += chunk;
      if (text.includes('\n') || text.length > MAX_OUTPUT_BYTES) socket.end();
    });
    socket.on('error', reject);
    socket.on('close', () => {
      resolve(text.split('\n')[0] ?? '');
    });
  });
}

async function check(): Promise<number> {
  const reply = JSON.parse(await ask(SOCKET, '{"verb":"nodes"}\n')) as Reply<{ nodes: readonly unknown[] }>;
  if (!reply.ok) {
    process.stdout.write(`broker: refused ${reply.refusal}: ${reply.detail}\n`);
    return 1;
  }
  process.stdout.write(`broker: ok, ${String(reply.value.nodes.length)} nodes\n`);
  return 0;
}

async function main(args: readonly string[]): Promise<number> {
  if (args.length > 1) return USAGE;
  try {
    switch (args[0]) {
      case undefined:
        return await serveOnce();
      case 'count-nodes':
        return await countNodes();
      case 'check':
        return await check();
      default:
        process.stderr.write('usage: anima-broker [count-nodes | check]\n');
        return USAGE;
    }
  } catch (error) {
    // Only the error's name: a parse error can quote Headscale output, which holds keys (LOG-007).
    process.stderr.write(`anima-broker: ${args[0] ?? 'serve'} failed (${error instanceof Error ? error.name : 'error'})\n`);
    return 1;
  }
}

if (import.meta.main) process.exit(await main(process.argv.slice(2)));
