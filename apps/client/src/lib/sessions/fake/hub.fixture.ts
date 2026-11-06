// Test support: a real `Hub` whose HTTP endpoint is a fake hub answering the session operations as K1 and K4
// describe, the Phase 2 additions included. It records every request. Close `hub` after each test.
// No vitest import here.
import { parse, type OperationDefinitionNode } from 'graphql';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { sessionsApi, type SessionsApi, type ServerCertificate } from '../api';
import type { SessionEndReason } from '../ending';

/** One request as the hub received it. */
export interface Sent {
  readonly operation: string;
  readonly query: string;
  readonly variables: Readonly<Record<string, unknown>>;
}

/** What the fake hub knows about its one device; tests change it between steps. */
export interface FakeDevice {
  id: string;
  /** `unserved`: a hub without K1 refuses the field, as the Phase 1 hub does; `gone`: the device was deleted. */
  transfer: { copy: boolean; paste: boolean } | 'unserved' | 'gone';
  /** The RDP certificate the device presents now (K4 §3). */
  certificate: ServerCertificate;
  /** The fingerprint `acceptServerCertificate` recorded, if any. */
  acceptedFingerprint: string | null;
  /** RDP `security: RDP` or `ignoreCertificate`: no accepted certificate needed (K4 §1). */
  certificateExempt: boolean;
  /** What `endSession` answers as the recorded reason (K4 §5). */
  endReason: SessionEndReason | null;
  /** A refusal for a credential or a test that quotes the sent secret back, to test that it is never shown. */
  refuseCredential: boolean;
  /** The K7 code of that refusal. */
  refusalCode: 'VALIDATION' | 'CONFLICT';
}

export const CERTIFICATE: ServerCertificate = {
  fingerprintSha256: 'sha256:6b:1f:9e:00:aa:bb:cc:dd',
  subject: 'CN=desk.lan',
  notAfter: '2027-11-04T10:00:00Z',
  firstSeenAt: '2026-11-04T10:00:00Z',
};

type Body = { readonly data: unknown } | { readonly errors: readonly unknown[] };

const error = (code: string, message: string): Body => ({ errors: [{ message, extensions: { code } }] });

function operationOf(query: string): string {
  const definition = parse(query).definitions[0] as OperationDefinitionNode | undefined;
  return definition?.name?.value ?? '';
}

/** `startSession`: a new ticket per call on the protocol's path (K4 §1), or the refusals K4 names. */
function ticket(device: FakeDevice, input: Record<string, unknown>, issued: number): Body {
  if (input.deviceId !== device.id) return error('NOT_FOUND', 'No such device.');
  const protocol = String(input.protocol);
  if (protocol === 'RDP' && device.acceptedFingerprint === null && !device.certificateExempt) {
    return error('CONFLICT', 'No server certificate is accepted yet.');
  }
  const n = String(issued);
  const startSession = {
    sessionId: `session-${n}`,
    token: `token-${n}-${'x'.repeat(36)}`,
    tunnelPath: protocol === 'SSH' ? '/terminal' : '/tunnel',
    expiresAt: '2026-11-04T10:00:30Z',
  };
  return { data: { startSession } };
}

/** `testConnection`; the K1 document also gets the server certificate for an RDP test (K4 §3). */
function tested(device: FakeDevice, operation: string, input: Record<string, unknown>): Body {
  if (device.refuseCredential) return quoting(device, input.password ?? input.credential);
  const phaseOne = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };
  if (operation === 'TestConnection') return { data: { testConnection: phaseOne } };
  const rdp = input.protocol === 'RDP';
  const accepted = device.acceptedFingerprint;
  return {
    data: {
      testConnection: {
        ...phaseOne,
        serverCertificate: rdp ? device.certificate : null,
        certificateChanged: rdp && accepted !== null && accepted !== device.certificate.fingerprintSha256,
      },
    },
  };
}

/** A refusal whose text quotes `secret`, as a careless hub might. */
function quoting(device: FakeDevice, secret: unknown): Body {
  const sent = JSON.stringify(secret);
  const fields = device.refusalCode === 'VALIDATION' ? [{ field: 'password', message: sent }] : undefined;
  return { errors: [{ message: `Refused ${sent}`, extensions: { code: device.refusalCode, fields } }] };
}

function credential(device: FakeDevice, operation: string, variables: Record<string, unknown>): Body {
  const field = operation === 'SetRdpCredential' ? 'setRdpCredential' : 'setVncCredential';
  if (device.refuseCredential) return quoting(device, variables.credential);
  return { data: { [field]: { credentialStored: true } } };
}

function accepted(device: FakeDevice, fingerprint: unknown): Body {
  if (fingerprint !== device.certificate.fingerprintSha256) return error('CONFLICT', 'That is not the certificate the hub saw.');
  device.acceptedFingerprint = fingerprint;
  return { data: { acceptServerCertificate: device.certificate } };
}

function ended(device: FakeDevice, id: unknown): Body {
  const session = {
    id,
    deviceId: device.id,
    deviceName: 'desk',
    protocol: 'RDP',
    startedAt: '2026-11-04T10:00:00Z',
    endedAt: '2026-11-04T10:05:00Z',
    endReason: device.endReason,
  };
  return { data: { endSession: session } };
}

function transferOf(device: FakeDevice, id: unknown): Body {
  if (device.transfer === 'unserved') return { errors: [{ message: 'Unknown field "transfer" on type "Device".' }] };
  const known = id === device.id && device.transfer !== 'gone';
  return { data: { device: known ? { transfer: device.transfer } : null } };
}

function answer(device: FakeDevice, request: Sent, issued: number): Body {
  const { operation, variables } = request;
  const input = (variables.input ?? {}) as Record<string, unknown>;
  switch (operation) {
    case 'StartSession':
      return ticket(device, input, issued);
    case 'DeviceTransfer':
      return transferOf(device, variables.id);
    case 'TestConnection':
    case 'TestRemoteConnection':
      return tested(device, operation, input);
    case 'SetRdpCredential':
    case 'SetVncCredential':
      return credential(device, operation, variables);
    case 'AcceptServerCertificate':
      return accepted(device, variables.fingerprintSha256);
    case 'EndSession':
      return ended(device, variables.id);
    default:
      return error('INTERNAL', `The fake hub does not answer ${operation}.`);
  }
}

/** A fake hub with one device, `desk` (`device-1`): copy and paste on, its certificate not yet accepted. */
export function fakeHub(changes: Partial<FakeDevice> = {}) {
  const device: FakeDevice = {
    id: 'device-1',
    transfer: { copy: true, paste: true },
    certificate: CERTIFICATE,
    acceptedFingerprint: null,
    certificateExempt: false,
    endReason: null,
    refuseCredential: false,
    refusalCode: 'VALIDATION',
    ...changes,
  };
  const sent: Sent[] = [];
  let issued = 0;
  const fetch: typeof globalThis.fetch = (_input, init) => {
    const body = JSON.parse(String(init?.body)) as { query: string; variables: Record<string, unknown> };
    const request: Sent = { operation: operationOf(body.query), ...body };
    sent.push(request);
    if (request.operation.startsWith('Start')) issued += 1;
    return Promise.resolve(new Response(JSON.stringify(answer(device, request, issued))));
  };
  const hub: Hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve('operator-token'), fetch, WebSocket: FAKE_WEBSOCKET });
  const api: SessionsApi = sessionsApi(hub);
  /** The requests named `operation`, in order. */
  const named = (operation: string) => sent.filter((request) => request.operation === operation);
  return { hub, api, device, sent, named };
}
