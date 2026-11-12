import { randomUUID } from 'node:crypto';
import { expect, type Page, type WebSocketRoute } from '@playwright/test';
import { instruction } from '../synthetic/tunnels';
import type { Screen } from './shots';
import { openDevices } from './steps';

// The session views: SSH in a terminal, VNC and RDP through Guacamole, the panels a session can show and the modals the
// gateway's answers open. The synthetic hub draws one colour and a prompt; these screens change what the tunnel says.

const HOST_KEY_CHANGED = 4001;
const CERTIFICATE_CHANGED = 4003;
const NORMAL_CLOSE = 1000;

type Serve = (socket: WebSocketRoute) => void;

/** Replaces the hub's answer on the tunnel `path` for this page. */
async function tunnelSays(page: Page, path: '/terminal' | '/tunnel', serve: Serve): Promise<void> {
  await page.routeWebSocket((url) => url.pathname === path, serve);
}

const closeWith = (code: number): Serve => (socket) => {
  void socket.close({ code, reason: 'Closed' });
};

const staysSilent: Serve = () => undefined;

/** The gateway asks for these parameters before it connects (GW-015). */
const asksFor = (...names: string[]): Serve => (socket) => {
  socket.send(instruction('', randomUUID()));
  socket.send(instruction('required', ...names));
};

async function connect(page: Page, button: string, tab: RegExp): Promise<void> {
  await openDevices(page);
  await page.getByRole('button', { name: button }).click();
  await expect(page.getByRole('tab', { name: tab })).toBeVisible();
}

const sshToGpuBox = (page: Page) => connect(page, 'SSH to gpu-box', /gpu-box/);

async function openTerminal(page: Page): Promise<void> {
  await sshToGpuBox(page);
  await expect(page.getByRole('group', { name: 'Terminal of gpu-box' }).locator('.xterm')).toBeVisible();
}

export const SESSION_SCREENS: readonly Screen[] = [
  { name: 'session-ssh-terminal', open: openTerminal, windowOnly: true },
  {
    name: 'session-ssh-connecting',
    open: async (page) => {
      await tunnelSays(page, '/terminal', staysSilent);
      await sshToGpuBox(page);
    },
    windowOnly: true,
  },
  {
    name: 'session-ssh-files-panel',
    open: async (page) => {
      await openTerminal(page);
      await page.getByRole('button', { name: 'Files', exact: true }).click();
    },
    windowOnly: true,
  },
  {
    name: 'session-ssh-ended',
    open: async (page) => {
      await tunnelSays(page, '/terminal', closeWith(NORMAL_CLOSE));
      await sshToGpuBox(page);
    },
    windowOnly: true,
  },
  {
    name: 'session-ssh-host-key-modal',
    open: async (page) => {
      await tunnelSays(page, '/terminal', closeWith(HOST_KEY_CHANGED));
      await sshToGpuBox(page);
      await expect(page.getByRole('alertdialog').or(page.getByRole('dialog')).first()).toBeVisible();
    },
    windowOnly: true,
  },
  { name: 'session-vnc-open', open: (page) => connect(page, 'VNC to mac-mini', /mac-mini/), windowOnly: true },
  { name: 'session-rdp-open', open: (page) => connect(page, 'RDP to win-desktop', /win-desktop/), windowOnly: true },
  {
    name: 'session-rdp-credential-modal',
    open: async (page) => {
      await tunnelSays(page, '/tunnel', asksFor('username', 'password'));
      await connect(page, 'RDP to win-desktop', /win-desktop/);
      await expect(page.getByRole('dialog').first()).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'session-vnc-credential-modal',
    open: async (page) => {
      await tunnelSays(page, '/tunnel', asksFor('password'));
      await connect(page, 'VNC to mac-mini', /mac-mini/);
      await expect(page.getByRole('dialog').first()).toBeVisible();
    },
    windowOnly: true,
  },
  {
    name: 'session-rdp-certificate-modal',
    open: async (page) => {
      await tunnelSays(page, '/tunnel', closeWith(CERTIFICATE_CHANGED));
      await connect(page, 'RDP to win-desktop', /win-desktop/);
      await page.waitForTimeout(1500);
    },
    windowOnly: true,
  },
  {
    name: 'session-two-tabs',
    open: async (page) => {
      await sshToGpuBox(page);
      await page.getByRole('link', { name: 'Devices', exact: true }).click();
      await page.getByRole('button', { name: 'VNC to mac-mini' }).click();
      await expect(page.getByRole('tab', { name: /mac-mini/ })).toBeVisible();
    },
    windowOnly: true,
  },
];
