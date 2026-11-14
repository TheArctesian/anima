import { describe, expect, it } from 'vitest';
import { formValues, type DeviceFormValues } from '$lib/devices/form/values';
import type { HubResult } from '$lib/hub/hub';
import { duration } from '$lib/motion/tokens';
import { device } from '../page.fixture.svelte';
import { SettingsDraft } from './draft.svelte';

const OK: HubResult<unknown> = { ok: true, value: null };

/**
 * A hub whose answers the test releases one at a time: `stored` is what it holds, each request waits in `waiting`
 * until `answer` lets the oldest one through.
 */
function slowHub() {
  let stored = formValues(device());
  const sent: DeviceFormValues[] = [];
  const waiting: ((result: HubResult<unknown>) => void)[] = [];
  return {
    sent,
    saved: () => stored,
    save: (values: DeviceFormValues) => {
      sent.push(values);
      return new Promise<HubResult<unknown>>((resolve) => {
        waiting.push((result) => {
          // The hub stores the trimmed name, as `updateInput` sends it.
          if (result.ok) stored = { ...values, name: values.name.trim(), credential: stored.credential, remoteSecrets: stored.remoteSecrets };
          resolve(result);
        });
      });
    },
    /** Lets the draft carry on until it waits for the hub again. */
    flush: () => new Promise((resolve) => setTimeout(resolve, 0)),
    /** Answers the oldest waiting request, then lets the draft carry on. */
    answer: async (result: HubResult<unknown> = OK) => {
      await new Promise((resolve) => setTimeout(resolve, 0));
      waiting.shift()?.(result);
      await new Promise((resolve) => setTimeout(resolve, 0));
    },
    get pending() {
      return waiting.length;
    },
  };
}

/** A clock whose waits run only when the test says. */
function manualClock() {
  const waits: { run: () => void; milliseconds: number; cancelled: boolean }[] = [];
  return {
    waits,
    later: (run: () => void, milliseconds: number) => {
      const wait = { run, milliseconds, cancelled: false };
      waits.push(wait);
      return () => {
        wait.cancelled = true;
      };
    },
  };
}

describe('device settings save on change (D17)', () => {
  it('D17: saves go one at a time, and the next is built on the device as the hub stored the one before', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.name = 'vault';
    void draft.commit('name');
    draft.values.host = 'vault.home.arpa';
    void draft.commit('host');
    await hub.flush();
    expect(hub.sent).toHaveLength(1);
    await hub.answer();
    expect(hub.sent).toHaveLength(2);
    expect([hub.sent[1]?.name, hub.sent[1]?.host]).toEqual(['vault', 'vault.home.arpa']);
  });

  it('D17: a change made again while its save still waits replaces that save, so a burst of steps sends two saves', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.ssh.port = '2201';
    void draft.commit('ssh.port');
    await hub.flush();
    for (const port of ['2202', '2203', '2204']) {
      draft.values.ssh.port = port;
      void draft.commit('ssh.port');
    }
    await hub.answer();
    await hub.answer();
    expect(hub.sent.map((values) => values.ssh.port)).toEqual(['2201', '2204']);
    expect(hub.pending).toBe(0);
  });

  it('D17: a switch flipped back while its save is on its way is saved back, not left on', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.ssh.enabled = false;
    void draft.commit('ssh.enabled');
    await hub.flush();
    draft.values.ssh.enabled = true;
    void draft.commit('ssh.enabled');
    await hub.answer();
    await hub.answer();
    expect(hub.sent.map((values) => values.ssh.enabled)).toEqual([false, true]);
    expect(hub.saved().ssh.enabled).toBe(true);
  });

});

describe('device settings: a refused change (D17, DEV-010, D-2)', () => {
  it('D17, DEV-010: a value that does not validate is never sent and stays shown, refused, with its reason', () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.ssh.port = 'abc';
    void draft.commit('ssh.port');
    expect(hub.sent).toEqual([]);
    expect(draft.values.ssh.port).toBe('abc');
    expect(draft.marks['ssh.port']).toBe('refused');
    expect(draft.errorOf('ssh.port')?.message).toBe('Enter a port from 1 to 65535.');
  });

  it('D17: a refused field does not hold back a change elsewhere, and is not sent with it', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.ssh.port = 'abc';
    void draft.commit('ssh.port');
    draft.values.name = 'vault';
    void draft.commit('name');
    await hub.answer();
    expect(hub.sent.map((values) => [values.name, values.ssh.port])).toEqual([['vault', '22']]);
    expect(draft.errorOf('ssh.port')).toBeDefined();
  });

  it('D17, D-2: a value the hub refuses stays shown and unsaved, and an error with no field is told beside the change', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.name = 'vault';
    void draft.commit('name');
    await hub.answer({ ok: false, error: { code: 'UNAVAILABLE', message: 'The database is unreachable.' } });
    expect(draft.values.name).toBe('vault');
    expect(hub.saved().name).toBe('nas');
    expect(draft.marks['name']).toBe('refused');
    expect(draft.errorOf('name')?.message).toBe('The database is unreachable.');
  });

  it('D17: a refused value put back as saved sends nothing and loses its reason', () => {
    const hub = slowHub();
    const draft = new SettingsDraft(hub.saved, hub.save, manualClock().later);
    draft.values.ssh.port = 'abc';
    void draft.commit('ssh.port');
    draft.values.ssh.port = '22';
    void draft.commit('ssh.port');
    expect(hub.sent).toEqual([]);
    expect(draft.errorOf('ssh.port')).toBeUndefined();
    expect(draft.marks['ssh.port']).toBeUndefined();
  });

});

describe('device settings: marks and secrets (D17, DEV-012)', () => {
  it('D17: the mark reads "Saving" until the hub answers, then "Saved", which fades after the highlight time', async () => {
    const hub = slowHub();
    const clock = manualClock();
    const draft = new SettingsDraft(hub.saved, hub.save, clock.later);
    draft.values.name = 'vault ';
    void draft.commit('name');
    expect(draft.marks['name']).toBe('saving');
    await hub.answer();
    expect(draft.marks['name']).toBe('saved');
    expect(draft.values.name).toBe('vault');
    expect(clock.waits.map((wait) => wait.milliseconds)).toEqual([duration.highlight]);
    clock.waits[0]?.run();
    expect(draft.marks['name']).toBe('faded');
  });

  it('DEV-012, D17: Save password checks a private key before it goes, and sends nothing for one that is not a key', async () => {
    const hub = slowHub();
    const draft = new SettingsDraft(() => ({ ...hub.saved(), ssh: { ...hub.saved().ssh, auth: 'PRIVATE_KEY' } }), hub.save, manualClock().later);
    const problem = await draft.saveSecret('ssh.credential', {
      credential: { password: '', privateKey: 'not a key', passphrase: '' },
      remoteSecrets: { rdpPassword: '', vncPassword: '' },
    });
    expect(problem).toContain('This is not a private key.');
    expect(hub.sent).toEqual([]);
  });
});
