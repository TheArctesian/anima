import { expect, it } from 'vitest';
import { comingSoon } from './coming';

it('R-223: an extension that is not built says "Coming soon" and what it will do', () => {
  expect(comingSoon('map')).toBe('Coming soon. It will show where you were, the places that you use, and where your sites and belongings are.');
  expect(comingSoon('model_gateway')).toBe('Coming soon. It will give each of your computers one address for AI model requests. The keys stay on the hub.');
});

it('R-223: an extension that the client has no sentence for says "Coming soon" alone', () => {
  expect(comingSoon('a-new-extension')).toBe('Coming soon');
});

it('R-223: no sentence gives a date', () => {
  for (const key of ['map', 'writing', 'secrets', 'people', 'money', 'history', 'health', 'documents', 'reminders', 'assistant_sessions', 'wardrobe', 'library', 'model_gateway']) {
    expect(comingSoon(key)).toMatch(/^Coming soon\. It will /);
    expect(comingSoon(key)).not.toMatch(/\d{4}|next|soon after|week|month/);
  }
});
