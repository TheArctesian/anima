import { configChanged, configInput } from './config-input';
import type { CreateCheckInput, UpdateCheckInput } from './hub-types';
import type { CheckFormValues } from './values';

const number = (text: string): number => Number(text.trim());

/** CHK-008, CHK-009: the create mutation input; every common setting is sent, so the hub applies no default of its own. */
export function createCheckInput(values: CheckFormValues, deviceId: string): CreateCheckInput {
  return {
    deviceId,
    name: values.name.trim(),
    config: configInput(values),
    intervalSeconds: number(values.intervalSeconds),
    retries: number(values.retries),
    retryIntervalSeconds: number(values.retryIntervalSeconds),
    degradedThresholdMs: number(values.degradedThresholdMs),
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
    ...(number(values.intervalSeconds) !== number(initial.intervalSeconds) ? { intervalSeconds: number(values.intervalSeconds) } : {}),
    ...(number(values.retries) !== number(initial.retries) ? { retries: number(values.retries) } : {}),
    ...(number(values.retryIntervalSeconds) !== number(initial.retryIntervalSeconds)
      ? { retryIntervalSeconds: number(values.retryIntervalSeconds) }
      : {}),
    ...(number(values.degradedThresholdMs) !== number(initial.degradedThresholdMs)
      ? { degradedThresholdMs: number(values.degradedThresholdMs) }
      : {}),
    ...(values.invert !== initial.invert ? { invert: values.invert } : {}),
  };
}
