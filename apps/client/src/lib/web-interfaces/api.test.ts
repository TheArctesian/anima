import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { HubError } from '$lib/hub/hub';
import { webInterfacesApi, type WebInterface } from './api';

const SAVED: WebInterface = {
  id: 'w1',
  deviceId: 'd1',
  label: 'Proxmox VE',
  url: 'https://pve.lan:8006',
  openIn: 'ANIMA_TAB',
  pinnedFingerprintSha256: null,
  pinnedAt: null,
};
const INPUT = { url: 'https://pve.lan:8006', label: 'Proxmox VE', openIn: 'ANIMA_TAB' } as const;

describe('webInterfacesApi (WEB-001, WEB-002)', () => {
  it('WEB-001: create and update send the input unchanged and return the web interface', async () => {
    const fake = fakeHub({
      CreateWebInterface: { createWebInterface: SAVED },
      UpdateWebInterface: { updateWebInterface: SAVED },
    });
    const api = webInterfacesApi(fake.hub);
    expect(await api.create('d1', INPUT)).toEqual({ ok: true, value: SAVED });
    expect(await api.update('w1', INPUT)).toEqual({ ok: true, value: SAVED });
    expect(fake.requests).toEqual([
      { operation: 'CreateWebInterface', variables: { deviceId: 'd1', input: INPUT } },
      { operation: 'UpdateWebInterface', variables: { id: 'w1', input: INPUT } },
    ]);
  });

  it('WEB-001: delete resolves to the deleted ID', async () => {
    const fake = fakeHub({ DeleteWebInterface: { deleteWebInterface: 'w1' } });
    expect(await webInterfacesApi(fake.hub).delete('w1')).toEqual({ ok: true, value: 'w1' });
    expect(fake.requests).toEqual([{ operation: 'DeleteWebInterface', variables: { id: 'w1' } }]);
  });

  it('WEB-002: accept makes a web interface and dismiss resolves to the check ID', async () => {
    const fake = fakeHub({
      AcceptWebInterfaceSuggestion: { acceptWebInterfaceSuggestion: SAVED },
      DismissWebInterfaceSuggestion: { dismissWebInterfaceSuggestion: 'c1' },
    });
    const api = webInterfacesApi(fake.hub);
    expect(await api.acceptSuggestion('c1')).toEqual({ ok: true, value: SAVED });
    expect(await api.dismissSuggestion('c1')).toEqual({ ok: true, value: 'c1' });
    expect(fake.requests.map((request) => request.variables)).toEqual([{ checkId: 'c1' }, { checkId: 'c1' }]);
  });

  it('WEB-001: a hub error passes through as a result', async () => {
    const refused: HubError = { code: 'VALIDATION', message: 'Check the form.', fields: [{ field: 'url', message: 'Not a URL.' }] };
    const fake = fakeHub({ CreateWebInterface: refused });
    expect(await webInterfacesApi(fake.hub).create('d1', INPUT)).toEqual({ ok: false, error: refused });
  });
});
