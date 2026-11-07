// The only place in the client that holds Nerd Font code points (UI-016).
// Source: glyphnames.json at Nerd Fonts commit f034eeafc0ccf79afa8dd952feb86b18e5d0469e.
export const iconCodePoints = {
  'nf-md-server': 0xf048b,
  'nf-md-access_point': 0xf0003,
  'nf-md-cog': 0xf0493,
  'nf-md-lan_connect': 0xf0318,
  'nf-md-lan_disconnect': 0xf0319,
  'nf-md-check_circle': 0xf05e0,
  'nf-md-close_circle': 0xf0159,
  'nf-md-alert': 0xf0026,
  'nf-md-alert_octagon': 0xf0029,
  'nf-md-wrench': 0xf05b7,
  'nf-md-help_circle': 0xf02d7,
  'nf-md-console': 0xf018d,
  'nf-md-plus': 0xf0415,
  'nf-md-pencil': 0xf03eb,
  'nf-md-delete': 0xf01b4,
  'nf-md-magnify': 0xf0349,
  'nf-md-sort': 0xf04ba,
  'nf-md-filter_variant': 0xf0236,
  'nf-md-chevron_down': 0xf0140,
  'nf-md-chevron_right': 0xf0142,
  'nf-md-close': 0xf0156,
  'nf-md-refresh': 0xf0450,
  'nf-md-pause': 0xf03e4,
  'nf-md-play': 0xf040a,
  'nf-md-key': 0xf0306,
  'nf-md-lock': 0xf033e,
  'nf-md-tag': 0xf04f9,
  'nf-md-web': 0xf059f,
  'nf-md-dots_vertical': 0xf01d9,
  'nf-md-folder': 0xf024b,
  'nf-md-file': 0xf0214,
  'nf-md-link': 0xf0337,
  'nf-md-upload': 0xf0552,
  'nf-md-download': 0xf01da,
  'nf-md-arrow_up': 0xf005d,
} as const;

export type IconName = keyof typeof iconCodePoints;

export function glyph(name: IconName): string {
  return String.fromCodePoint(iconCodePoints[name]);
}
