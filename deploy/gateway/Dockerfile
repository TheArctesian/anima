# Gateway image (GW-001, RA-011): the official guacd plus the OpenDyslexic font and its licence.
# The `fonts` build context is apps/client/src/styles/fonts (see deploy/compose.yaml).
# The drives root (RA-021) is created here, owned by guacd and mode 0700, so a new named volume
# copies that owner and mode. Each device gets one folder under it, made by guacd.
FROM guacamole/guacd:1.6.0

USER root
COPY --from=fonts OpenDyslexicMNerdFontMono-Regular.otf LICENSE-OpenDyslexic.txt /usr/share/fonts/opendyslexic/
RUN chmod 0644 /usr/share/fonts/opendyslexic/* \
    && if command -v fc-cache >/dev/null 2>&1; then fc-cache -f; fi \
    && mkdir -p /var/lib/guacd/drives \
    && chown guacd:guacd /var/lib/guacd/drives \
    && chmod 0700 /var/lib/guacd/drives
USER guacd
