/** Turn a repo-relative glob into a RegExp. `**` crosses folders, `*` and `?` do not. */
export function globToRegExp(glob: string): RegExp {
  let source = "";
  for (let i = 0; i < glob.length; i++) {
    const char = glob.charAt(i);
    if (char === "*" && glob.charAt(i + 1) === "*") {
      source += ".*";
      i++;
    } else if (char === "*") {
      source += "[^/]*";
    } else if (char === "?") {
      source += "[^/]";
    } else {
      source += char.replace(/[.+^${}()|[\]\\]/g, "\\$&");
    }
  }
  return new RegExp(`^${source}$`);
}

/** Read `generated.txt`: one glob per line, `#` starts a comment line. */
export function parseGeneratedList(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"));
}

export function withoutGenerated(paths: string[], globs: string[]): string[] {
  const patterns = globs.map(globToRegExp);
  return paths.filter((path) => !patterns.some((pattern) => pattern.test(path)));
}
