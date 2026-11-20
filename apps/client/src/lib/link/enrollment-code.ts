/** Recognize the invitation envelope; the native link validates and redeems its contents. */
export function enrollmentCode(text: string): string | null {
  const code = text.trim();
  return code.length <= 4096 && /^anima-invite\.1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_.-]+$/.test(code) ? code : null;
}
