# Anima logo

`logo.png` is the approved recursive doorway artwork: Nord blue frames progress
toward cyan, with dithered recessed faces and a left-hinged, dithered door opening
onto charcoal. It replaces the temporary circular desktop mark (UI-001).

`social-preview.png` places the artwork on a 1280 × 640 charcoal canvas for
GitHub's repository social preview, preserving the square logo's proportions.

The client uses `apps/client/static/logo.png`, `favicon.png` and
`apple-touch-icon.png`. The desktop bundle uses `apps/desktop/src-tauri/icons/`.
The former `apps/desktop/icon-source.svg` was a placeholder and is retired.

To regenerate desktop icons from the approved source, run from the repository root:

```sh
bun install --frozen-lockfile
bun run --cwd apps/desktop tauri icon ../../design/brand/logo.png --output src-tauri/icons
```

Keep the six existing desktop icon files; the generator also emits mobile and
Windows Store assets that are not used by the current bundle.

The source was made with the built-in image generation tool. Final edit prompt:
match the terminal door to the recessed side-face blue-teal colour and fine pixel
dithering; preserve the recursive geometry, blue-to-cyan frames, left hinge and
charcoal opening.
