import type { CheckConfigFields, CheckFields, CheckType } from './model/hub-types';
import { validateCheck } from './model/validate';
import { checkValues, type CheckFormValues } from './model/values';

export function saved(type: CheckType, config: CheckConfigFields, change: Partial<CheckFields> = {}): CheckFields {
  return {
    id: 'chk-1',
    type,
    name: 'check',
    intervalSeconds: 60,
    retries: 3,
    retryIntervalSeconds: 20,
    degradedThresholdMs: 1000,
    invert: false,
    config,
    ...change,
  };
}

export const HTTP_SAVED = saved('HTTP', {
  __typename: 'HttpConfig',
  url: 'https://example.com/health',
  method: 'GET',
  headers: [
    { name: 'X-Env', value: 'prod' },
    { name: 'Authorization', value: null },
  ],
  body: null,
  maxRedirects: 10,
  acceptedStatus: [{ from: 200, to: 299 }],
  keyword: null,
  jsonQuery: null,
  jsonExpected: null,
  authKind: 'BASIC',
  verifyTls: true,
  timeoutMs: 10000,
});

export function fresh(type: CheckType, change: (values: CheckFormValues) => void = () => undefined): CheckFormValues {
  const values = checkValues(null, type);
  values.name = 'a check';
  change(values);
  return values;
}

export function fieldsOf(values: CheckFormValues, initial: CheckFormValues | null = null): string[] {
  return validateCheck(values, initial).map((error) => error.field);
}

