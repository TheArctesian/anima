import { expect, it } from 'vitest';
import { willDo } from './coming';

it('R-223: an extension that is not built says what it will do', () => {
  expect(willDo('map')).toBe('It will show where you were, the places that you use, and where your sites and belongings are.');
  expect(willDo('model_gateway')).toBe('It will give each of your computers one address for AI model requests. The keys stay on the hub.');
});

it('R-223: an extension that the client has no sentence for says nothing more than its tag', () => {
  expect(willDo('a-new-extension')).toBe('');
});

it('R-223: no sentence gives a date', () => {
  for (const key of ['map', 'writing', 'secrets', 'people', 'money', 'history', 'health', 'documents', 'reminders', 'assistant_sessions', 'wardrobe', 'library', 'model_gateway']) {
    expect(willDo(key)).toMatch(/^It will /);
    expect(willDo(key)).not.toMatch(/\d{4}|next|soon after|week|month/);
  }
});
