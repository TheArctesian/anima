import { expect, it } from 'vitest';
import { fieldMessages } from './form';

it('DEV-011: only the fields the form has get a message; an unknown field gets none', () => {
  const messages = fieldMessages([
    { field: 'nickname', message: 'Not a field.' },
    { field: 'totpCode', message: 'The code is not accepted.' },
  ]);
  expect(messages).toEqual({ totpCode: 'The code is not accepted.' });
});
