import type { CheckDefaults } from '$lib/checks/api';
import type { CheckDefaultsInput } from '$lib/hub/generated/graphql';
import { LIMITS, toNumber } from '../form/model/range';
import type { CheckSettingsText } from '../form/model/validate';
import { defaultsText } from '../form/model/values';

/** The four settings of D-26, in form order, with the label the check form uses and the hub's bounds for each. */
export const SETTING_FIELDS = [
  { key: 'intervalSeconds', label: 'Interval (seconds)', bounds: LIMITS.intervalSeconds },
  { key: 'retries', label: 'Retries before Down', bounds: LIMITS.retries },
  { key: 'retryIntervalSeconds', label: 'Retry interval (seconds)', bounds: LIMITS.retryIntervalSeconds },
  { key: 'degradedThresholdMs', label: 'Degraded threshold (ms)', bounds: LIMITS.degradedThresholdMs },
] as const satisfies readonly { key: keyof CheckDefaults; label: string; bounds: readonly [number, number] }[];

/** The values to send for settings that passed `settingsErrors`. */
export function defaultsInput(text: CheckSettingsText): CheckDefaultsInput {
  return {
    intervalSeconds: toNumber(text.intervalSeconds),
    retries: toNumber(text.retries),
    retryIntervalSeconds: toNumber(text.retryIntervalSeconds),
    degradedThresholdMs: toNumber(text.degradedThresholdMs),
  };
}

export type SettingKey = (typeof SETTING_FIELDS)[number]['key'];

/** D-26: `defaults` for the settings of a new check's form that the operator has not touched; a touched one is left out. */
export function untouchedSettings(touched: ReadonlySet<SettingKey>, defaults: CheckDefaults): Partial<CheckSettingsText> {
  const wanted = defaultsText(defaults);
  const patch: Partial<CheckSettingsText> = {};
  for (const { key } of SETTING_FIELDS) if (!touched.has(key)) patch[key] = wanted[key];
  return patch;
}
