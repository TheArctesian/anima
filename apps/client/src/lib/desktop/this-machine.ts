import { invoke, isTauri } from '@tauri-apps/api/core';
import type { OperatingSystem } from '$lib/hub/generated/graphql';
import { isOnThisComputer, type Hub } from '$lib/hub/hub';
import { vpnApi } from '$lib/vpn/api';

/** The computer the desktop app runs on, as the shell reads it (R-058, R-060). Nothing here is logged (LOG-007). */
export interface ThisMachine {
  /** The computer name; empty when the OS gives none. */
  readonly name: string;
  /** The OS user name of the app; empty when the OS gives none. */
  readonly user: string;
  readonly os: Extract<OperatingSystem, 'MACOS' | 'LINUX' | 'WINDOWS'>;
  /** Its IPv4 address on the Anima.ing VPN, from the VPN client logged in to the hub's login server (NET-001, ADR-005). */
  readonly vpnAddress: string;
  /** Whether SSH (TCP 22) accepted a connection on `vpnAddress`. */
  readonly sshListening: boolean;
}

const OCTET = '(?:25[0-5]|2[0-4]\\d|1\\d\\d|[1-9]?\\d)';
const IPV4 = new RegExp(`^${OCTET}(?:\\.${OCTET}){3}$`);

/** Whether `text` is an IPv4 address inside 100.64.0.0/10, the Anima.ing VPN's range (NET-001). */
function isVpnAddress(text: string): boolean {
  if (!IPV4.test(text)) return false;
  const [first, second] = text.split('.').map(Number);
  return first === 100 && second !== undefined && second >= 64 && second <= 127;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * The shell's answer as a `ThisMachine`, or `null` when it is not one or has no VPN address. The answer crosses
 * a process boundary, so every field is checked.
 */
function thisMachineFrom(answer: unknown): ThisMachine | null {
  if (!isRecord(answer)) return null;
  const { name, user, os, vpnAddress, sshListening } = answer;
  if (typeof name !== 'string' || typeof user !== 'string' || typeof sshListening !== 'boolean') return null;
  if (os !== 'MACOS' && os !== 'LINUX' && os !== 'WINDOWS') return null;
  if (typeof vpnAddress !== 'string' || !isVpnAddress(vpnAddress)) return null;
  return { name, user, os, vpnAddress, sshListening };
}

/**
 * This computer, read by the desktop shell. The shell takes the VPN address only from a VPN client logged in to
 * `hub`'s login server, so the hub is asked for it first (ADR-005). `null` outside the desktop app, when the hub
 * does not name its login server, when the shell fails, or when the computer is not on the VPN: in each case there is
 * nothing to fill the Add device form with, so the form opens empty as before. The shell's error carries only a code,
 * and none of them changes that outcome.
 *
 * A hub at a port of this computer is not reached over the VPN (a hosted hub is reached through the link). For such a
 * hub nothing is asked: the shell's read runs the VPN client, which starts the Tailscale application on a Mac where it
 * was quit (DEF-005).
 */
export async function readThisMachine(hub: Hub): Promise<ThisMachine | null> {
  if (!isTauri() || isOnThisComputer(hub.address)) return null;
  const loginServer = await vpnApi(hub).loginServer();
  if (!loginServer.ok) return null;
  try {
    return thisMachineFrom(await invoke('this_machine_read', { loginServer: loginServer.value }));
  } catch {
    return null;
  }
}
