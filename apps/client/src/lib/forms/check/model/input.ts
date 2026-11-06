import { configChanged, configInput } from './config-input';
import type { CreateCheckInput, UpdateCheckInput } from './hub-types';
import { toNumber } from './range';
import type { CheckFormValues } from './values';


/** CHK-008, CHK-009: the create mutation input; every common setting is sent, so the hub applies no default of its own. */
export function createCheckInput(values: CheckFormValues, deviceId: string): CreateCheckInput {
  return {
    deviceId,
    name: values.name.trim(),
    config: configInput(values),
    intervalSeconds: toNumber(values.intervalSeconds),
    retries: toNumber(values.retries),
    retryIntervalSeconds: toNumber(values.retryIntervalSeconds),
    degradedThresholdMs: toNumber(values.degradedThresholdMs),
    invert: values.invert,
  };
}

/**
 * CHK-009: only the fields that differ from `initial`; an unchanged form gives `{}`. The type is
 * that of `initial` and never changes after create. The config goes whole (the hub replaces it) and
 * only when it changed, so an edit of the name or interval never touches a stored secret.
 */
export function updateCheckInput(initial: CheckFormValues, values: CheckFormValues): UpdateCheckInput {
  return {
    ...(values.name.trim() !== initial.name.trim() ? { name: values.name.trim() } : {}),
    ...(configChanged(initial, values) ? { config: configInput({ ...values, type: initial.type }) } : {}),
    ...(toNumber(values.intervalSeconds) !== toNumber(initial.intervalSeconds) ? { intervalSeconds: toNumber(values.intervalSeconds) } : {}),
    ...(toNumber(values.retries) !== toNumber(initial.retries) ? { retries: toNumber(values.retries) } : {}),
    ...(toNumber(values.retryIntervalSeconds) !== toNumber(initial.retryIntervalSeconds)
      ? { retryIntervalSeconds: toNumber(values.retryIntervalSeconds) }
      : {}),
    ...(toNumber(values.degradedThresholdMs) !== toNumber(initial.degradedThresholdMs)
      ? { degradedThresholdMs: toNumber(values.degradedThresholdMs) }
      : {}),
    ...(values.invert !== initial.invert ? { invert: values.invert } : {}),
  };
}
