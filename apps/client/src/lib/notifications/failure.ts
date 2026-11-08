// Fixed sentences for a refused notification call. The hub's own text is never shown here: a fixed sentence is what
// the tests can hold to, and it can never carry a secret the operator typed or a target's reply (LOG-007, F-8).
import type { HubError } from '$lib/hub/hub';

/** What the operator was doing, for the sentence of a code that has no sentence of its own. */
type Action = 'save' | 'delete' | 'test' | 'settings';

const OTHER: Record<Action, string> = {
  save: 'Channel not saved. Try again.',
  delete: 'The hub did not delete the channel. Try again.',
  test: 'Anima could not send the test. Try again.',
  settings: 'The hub did not save the settings. Try again.',
};

/** Input names the hub may list in a VALIDATION error, by the last part of their path, as the form labels them. */
// A map, not an object, so a hub path such as `constructor` finds nothing.
const FIELD_LABELS = new Map<string, string>([
  ['name', 'Name'],
  ['url', 'URL'],
  ['headers', 'Headers'],
  ['server', 'Server'],
  ['topic', 'Topic'],
  ['token', 'Access token'],
  ['priority', 'Priority'],
  ['host', 'SMTP host'],
  ['port', 'Port'],
  ['username', 'User name'],
  ['password', 'Password'],
  ['from', 'From'],
  ['to', 'To'],
  ['urls', 'Apprise URLs'],
  ['key', 'Configuration key'],
  ['tags', 'Tags'],
  ['title', 'Template title'],
  ['body', 'Template body'],
  ['certificateDays', 'Certificate expiry days'],
]);

/** The labels of the fields the hub refused, from its field paths; a path the form does not know adds nothing. */
function refusedLabels(error: HubError): string[] {
  if (error.code !== 'VALIDATION') return [];
  const labels = error.fields.map((field) => FIELD_LABELS.get(field.field.split('.').at(-1) ?? '') ?? '');
  return [...new Set(labels.filter((label) => label !== ''))];
}

export function failureText(error: HubError, action: Action): string {
  switch (error.code) {
    case 'VALIDATION': {
      const labels = refusedLabels(error);
      return labels.length === 0 ? 'The hub refused what was entered.' : `The hub refused: ${labels.join(', ')}.`;
    }
    case 'CONFLICT':
      return action === 'save' ? 'Name in use. Choose another.' : OTHER[action];
    case 'NOT_FOUND':
      return 'This channel no longer exists.';
    case 'FORBIDDEN':
      return 'Not allowed for this sign-in.';
    case 'unreachable':
    case 'malformed':
      // These two are the client's own fixed sentences (`$lib/hub/error`).
      return error.message;
    default:
      return OTHER[action];
  }
}
