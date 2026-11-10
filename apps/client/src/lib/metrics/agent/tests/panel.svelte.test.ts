// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CODE, settle, ENROLMENT, button, click, command, enrolment, pass, setUp, show, tearDown, text } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

beforeEach(setUp);
afterEach(tearDown);

describe('Install agent: before an agent (MET-019)', () => {
  it('MET-019: a device not using the agent offers Install agent and asks the hub for nothing yet', async () => {
    const { fake } = await show({});
    expect(text()).toContain('Install agent');
    expect(fake.requests).toEqual([]);
    expect(command()).toBe('');
  });

  it('MET-019, MET-012: Install agent asks for a code for this device and shows it with the time left and the command', async () => {
    const { fake } = await show(ENROLMENT);
    await click('Install agent');
    expect(fake.requests).toEqual([{ operation: 'EnrolAgent', variables: { deviceId: 'd1' } }]);
    expect(document.querySelector('.code')?.textContent).toBe(CODE);
    expect(text()).toContain('expires in 15:00');
    expect(command()).toContain('anima-agent-linux-x86_64');
    expect(command()).toContain(`'${CODE}'`);
    expect(command()).toContain("sha256sum -c -");
  });

  it('MET-019: the chosen target changes the command, and the browser platform is only the default', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    const picker = document.querySelector('select');
    if (!picker) throw new Error('no target picker');
    picker.value = 'macos-aarch64';
    picker.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect(command()).toContain('anima-agent-macos-aarch64');
    expect(command()).toContain('shasum -a 256 -c -');
  });

});

describe('Install agent: the command and copying (MET-019)', () => {
  it('MET-019: the Copy command button copies exactly the command shown', async () => {
    const { copied } = await show(ENROLMENT);
    await click('Install agent');
    await click('Copy command');
    expect(copied).toEqual([command()]);
    expect(text()).toContain('Copied.');
  });

  it('MET-019: a refused copy says so and leaves the command to select', async () => {
    await show(ENROLMENT, { copyFails: true });
    await click('Install agent');
    await click('Copy command');
    expect(text()).toContain('could not be copied');
    expect(command()).toContain(CODE);
  });

  it('MET-019, D-10 c: with no release only the enrol command shows, with the note', async () => {
    await show(ENROLMENT, { release: null });
    await click('Install agent');
    expect(command()).toBe("anima-agent enrol --hub 'https://hub.test' -");
    expect(text()).toContain('paste the code at its prompt');
    expect(document.querySelector('select')).toBeNull();
  });

  it('MET-019: a code the agent would not accept shows no command, only the reason', async () => {
    await show(enrolment("x'; id; '"));
    await click('Install agent');
    expect(command()).toBe('');
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('no install command');
  });

  it('MET-012: a hub refusal is shown and the button stays', async () => {
    await show({ EnrolAgent: { code: 'FORBIDDEN', message: 'Not allowed.' } });
    await click('Install agent');
    expect(document.querySelector('[role="alert"]')?.textContent).toBe('Not allowed.');
    expect(text()).toContain('Install agent');
  });
});

describe('Install agent: the code does not outlive its use (MET-012, LOG-007)', () => {
  it('MET-012: when the 15 minutes pass the code and the command leave the page', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    await pass(899);
    expect(text()).toContain('expires in 0:01');
    expect(command()).toContain(CODE);
    await pass(1);
    expect(text()).not.toContain(CODE);
    expect(command()).toBe('');
    expect(text()).toContain('The code expired');
    expect(text()).toContain('Install agent');
  });

  it('MET-012: Close clears the code at once', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    await click('Close');
    expect(text()).not.toContain(CODE);
    expect(command()).toBe('');
  });

  it('LOG-007: the code is not in storage, the URL or the console', async () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((method) => vi.spyOn(console, method));
    await show(ENROLMENT);
    await click('Install agent');
    expect(JSON.stringify({ ...localStorage })).not.toContain(CODE);
    expect(JSON.stringify({ ...sessionStorage })).not.toContain(CODE);
    expect(location.href).not.toContain(CODE);
    expect(JSON.stringify(spies.map((spy) => spy.mock.calls))).not.toContain(CODE);
    for (const spy of spies) spy.mockRestore();
  });
});

describe('Install agent: review round 1 (MET-012, MET-013, MET-019)', () => {
  it('MET-012: a second click on Install agent while the first request is in flight sends no second EnrolAgent', async () => {
    const { fake } = await show(ENROLMENT);
    button('Install agent').click();
    button('Install agent').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'EnrolAgent')).toHaveLength(1);
  });

  it('MET-013: after one confirmed revoke, a second Revoke agent can be confirmed', async () => {
    const { fake } = await show(
      {
        AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale: false } },
        RevokeAgentToken: { revokeAgentToken: { id: 'd1', metricsSource: 'AGENT', metricsStale: true } },
      },
      { source: 'AGENT' },
    );
    await click('Revoke agent');
    await click('Revoke agent', '[role="dialog"]');
    await click('Revoke agent');
    await click('Revoke agent', '[role="dialog"]');
    expect(fake.requests.filter((request) => request.operation === 'RevokeAgentToken')).toHaveLength(2);
  });

  it('MET-019: curl is held to https, redirects included', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    expect(command()).toContain("--proto '=https' --proto-redir '=https'");
  });

  it('MET-012: a code the hub dated hours ahead is still cleared within 15 minutes', async () => {
    await show({ EnrolAgent: { enrolAgent: { code: CODE, expiresAt: '2026-11-10T10:00:00Z' } } });
    await click('Install agent');
    expect(text()).toContain('expires in 15:00');
    await pass(15 * 60 + 1);
    expect(text()).not.toContain(CODE);
  });

  it('MET-019: with a release and no computer chosen the note asks for the choice', async () => {
    Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Win32' });
    Object.defineProperty(navigator, 'userAgent', { configurable: true, value: 'Mozilla/5.0 (Windows NT 10.0)' });
    await show(ENROLMENT);
    await click('Install agent');
    expect(text()).toContain('Choose the computer above');
    expect(text()).not.toContain('No download is published');
  });
});
