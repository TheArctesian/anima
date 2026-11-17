import { expect, it } from 'vitest';
import { attempt } from '../sources.fixture';
import { describeAttempt, githubAddress } from './attempt';

const TASK = 'https://github.com/settings/apps/anima-k3f9/advanced';
const TRY = 'Delete any new App on GitHub that starts with "anima-".';

it('PRJ-039: WAITING_FOR_BROWSER says GitHub is opening and offers Continue in browser and Cancel', () => {
  const view = describeAttempt(attempt());
  expect(view).toMatchObject({ line: 'Opening GitHub in your browser.', continueInBrowser: true, cancel: true, start: false });
});

it('PRJ-039: AT_GITHUB tells the operator to press Create, about a taken name and about the password', () => {
  const view = describeAttempt(attempt({ phase: 'AT_GITHUB' }));
  expect(view.line).toBe(
    'Waiting for you on GitHub. Press "Create GitHub App". If the name is taken, change it there and press again. GitHub may ask for your password first.',
  );
  expect(view.continueInBrowser && view.cancel).toBe(true);
});

it('PRJ-039: APP_CREATED names the App and its owner and offers Install on GitHub', () => {
  const view = describeAttempt(attempt({ phase: 'APP_CREATED', appSlug: 'anima-k3f9', ownerLogin: 'octo', installUrl: 'https://github.com/apps/anima-k3f9/installations/new' }));
  expect(view).toMatchObject({
    line: 'The App anima-k3f9 is made for octo. Press Install on GitHub.',
    installUrl: 'https://github.com/apps/anima-k3f9/installations/new',
    cancel: true,
    continueInBrowser: false,
  });
});

it('PRJ-041: an installUrl or appSettingsUrl that is not on github.com gives no action', () => {
  expect(describeAttempt(attempt({ phase: 'APP_CREATED', installUrl: 'https://github.com.evil.example/x' })).installUrl).toBeNull();
  expect(describeAttempt(attempt({ phase: 'FAILED', failure: 'KEY_REFUSED', appSettingsUrl: 'http://github.com/x' })).appSettingsUrl).toBeNull();
  expect(githubAddress('javascript:alert(1)')).toBeNull();
});

it('PRJ-039: CONNECTED says the first sync started, or that Projects is off', () => {
  expect(describeAttempt(attempt({ phase: 'CONNECTED', account: { health: 'STARTING', pausedBy: null } }))).toMatchObject({
    line: 'GitHub is connected. The first sync has started.',
    cancel: false,
    start: false,
  });
  expect(describeAttempt(attempt({ phase: 'CONNECTED', account: { health: 'PAUSED', pausedBy: 'EXTENSION_OFF' } })).line).toBe(
    'GitHub is connected. Set Projects on in Settings > Extensions to read it.',
  );
});

it.each([
  ['EXPIRED', null, 'The connection timed out. Press Connect GitHub.'],
  ['EXPIRED', TASK, 'The App anima-k3f9 was not installed in time. Delete it on GitHub, then start again.'],
  ['EXCHANGE_REFUSED', null, `GitHub did not accept the reply from its page. Start again. ${TRY}`],
  ['GITHUB_UNREACHABLE', null, `The hub could not reach GitHub. Start again. ${TRY}`],
  ['REPLY_UNUSABLE', TASK, "GitHub's reply could not be used. Delete the App anima-k3f9 on GitHub, then start again."],
  ['KEY_REFUSED', TASK, "GitHub refused the new App's key. Delete the App anima-k3f9 on GitHub, then start again."],
] as const)('PRJ-039: FAILED with %s and settings %s gives its words', (failure, settings, line) => {
  const view = describeAttempt(attempt({ phase: 'FAILED', failure, appSlug: 'anima-k3f9', appSettingsUrl: settings }));
  expect(view.line).toBe(line);
  expect(view.appSettingsUrl).toBe(settings);
  expect(view.start).toBe(true);
});

it('PRJ-039: CANCELLED with an App says it is still on GitHub; without one it says Cancelled', () => {
  expect(describeAttempt(attempt({ phase: 'CANCELLED', appSlug: 'anima-k3f9', appSettingsUrl: TASK })).line).toBe('Cancelled. The App anima-k3f9 is still on GitHub. Delete it there.');
  expect(describeAttempt(attempt({ phase: 'CANCELLED' }))).toMatchObject({ line: 'Cancelled.', start: true, appSettingsUrl: null });
});

it('PRJ-041: an attempt that an AI agent started begins its line with that fact', () => {
  expect(describeAttempt(attempt({ startedBy: 'AGENT' })).line).toBe('An AI agent started this. Opening GitHub in your browser.');
});
