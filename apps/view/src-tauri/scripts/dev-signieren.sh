#!/bin/sh
# Cargo-Runner (siehe .cargo/config.toml): signiert das gebaute Programm und startet es.
programm="$1"; shift
if [ -n "$PSV_SIGN_IDENTITY" ] && ! printf %s "$PSV_SIGN_IDENTITY" | grep -q XXX; then
  codesign --force --sign "$PSV_SIGN_IDENTITY" "$programm" 2>/dev/null \
    || echo "dev-signieren: Signieren fehlgeschlagen – starte unsigniert" >&2
fi
exec "$programm" "$@"
