// WCAG 2.x contrast and the light-theme derivation (UI-005, D-27). Pure maths.

/** A colour written `#RRGGBB`. */
export type HexColour = `#${string}`;

/** A colour string that is not `#RRGGBB`. */
export class MalformedColourError extends Error {
  constructor(readonly colour: string) {
    super(`not a #RRGGBB colour: ${colour}`);
    this.name = 'MalformedColourError';
  }
}

/** No whole-percent mix reaches the wanted contrast. */
export class UnreachableContrastError extends Error {
  constructor(
    readonly colour: HexColour,
    readonly target: HexColour,
    readonly minimum: number,
  ) {
    super(`${colour} mixed toward ${target} never reaches ${String(minimum)}:1`);
    this.name = 'UnreachableContrastError';
  }
}

type Channels = readonly [number, number, number];

const HEX_COLOUR = /^#[0-9a-fA-F]{6}$/;
const FLARE = 0.05;

function channelsOf(colour: string): Channels {
  if (!HEX_COLOUR.test(colour)) throw new MalformedColourError(colour);
  return [1, 3, 5].map((start) => parseInt(colour.slice(start, start + 2), 16)) as unknown as Channels;
}

function linear(channel: number): number {
  const unit = channel / 255;
  return unit <= 0.04045 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance, 0 (black) to 1 (white). */
export function luminance(colour: string): number {
  const [red, green, blue] = channelsOf(colour).map(linear) as unknown as Channels;
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

/** WCAG contrast ratio, 1 to 21. The order of the colours does not matter. */
export function contrastRatio(first: string, second: string): number {
  const [a, b] = [luminance(first), luminance(second)];
  return (Math.max(a, b) + FLARE) / (Math.min(a, b) + FLARE);
}

/** The colour `fraction` (0 to 1) of the way from `from` to `toward`, mixed per sRGB channel. */
export function mixColour(from: string, toward: string, fraction: number): HexColour {
  const [start, end] = [channelsOf(from), channelsOf(toward)];
  const hex = start.map((channel, index) => {
    const mixed = Math.round(channel + ((end[index] ?? 0) - channel) * fraction);
    return mixed.toString(16).padStart(2, '0').toUpperCase();
  });
  return `#${hex.join('')}`;
}

/** The derived light-mode colour and how far it was mixed. */
export interface DerivedColour {
  readonly colour: HexColour;
  readonly mixPercent: number;
}

/** The least whole-percent mix of `colour` toward `target` whose contrast on `background` reaches `minimum`. */
export function deriveToContrast(
  colour: HexColour,
  target: HexColour,
  background: HexColour,
  minimum: number,
): DerivedColour {
  for (let mixPercent = 0; mixPercent <= 100; mixPercent += 1) {
    const mixed = mixColour(colour, target, mixPercent / 100);
    if (contrastRatio(mixed, background) >= minimum) return { colour: mixed, mixPercent };
  }
  throw new UnreachableContrastError(colour, target, minimum);
}

/**
 * The greatest whole-percent mix of `colour` toward `target` that still reaches `minimum` on `background`: the lightest
 * (or deepest) step that keeps the text legible. Used where a tint must stay visibly apart from its base (UI-002).
 */
export function largestMixToContrast(
  colour: HexColour,
  target: HexColour,
  background: HexColour,
  minimum: number,
): DerivedColour {
  for (let mixPercent = 100; mixPercent >= 0; mixPercent -= 1) {
    const mixed = mixColour(colour, target, mixPercent / 100);
    if (contrastRatio(mixed, background) >= minimum) return { colour: mixed, mixPercent };
  }
  throw new UnreachableContrastError(colour, target, minimum);
}
