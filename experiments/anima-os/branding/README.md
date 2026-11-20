# Anima.ing image assets

`logo.png` is the approved repository asset from `design/brand/logo.png`.
The Calamares and terminal palettes come from `design/tokens/palettes/nord.tokens.json`
and `design/tokens/terminal.tokens.json`. Body text uses Nord snow white after the
operator's explicit readability correction on 2026-11-20.

`hero.png` is a deterministic still of the existing anima-site particle hero,
rendered from `src/lib/story/field.ts` at site commit
`3d71012f754e7d76d677cbee66d6358f004a84d2`. Its background is the site's graphite base.
The source site is not modified. Regenerate with:

```sh
bun experiments/anima-os/branding/render-hero.ts /path/to/anima-site > /tmp/anima-hero.svg
rsvg-convert /tmp/anima-hero.svg -o experiments/anima-os/branding/hero.png
```

The fullscreen installer blends this still behind welcome/completion content with a slow drift; reduced motion disables that animation. Automatic screensaver, locking and
sleep are disabled during installation. This does not add a desktop to the installed server.

`wordmark.svg` is the operator-supplied vector wordmark. `settings.svg`, `back.svg`, `next.svg`, `power.svg` and `restart.svg` use IBM Carbon paths (Apache-2.0), matching `apps/client/src/lib/icons/shapes.ts`. The current installer uses Graphite #1B1D21 and frost #88C0D0 with IBM Plex Sans.
