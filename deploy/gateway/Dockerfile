# Gateway image (GW-001, RA-011): the official guacd plus the OpenDyslexic font and its licence.
# The `fonts` build context is apps/client/src/styles/fonts (see deploy/compose.yaml).
FROM guacamole/guacd:1.6.0

USER root
COPY --from=fonts OpenDyslexicMNerdFontMono-Regular.otf LICENSE-OpenDyslexic.txt /usr/share/fonts/opendyslexic/
RUN chmod 0644 /usr/share/fonts/opendyslexic/* \
    && if command -v fc-cache >/dev/null 2>&1; then fc-cache -f; fi
USER guacd
