#!/usr/bin/env bash
# CubeCheck self-extracting installer/launcher (KIND=@KIND@ VERSION=@VERSION@)
# chmod +x this file, then: ./CubeCheck-@VERSION@-@KIND@.sh
set -euo pipefail
SKIP=@SKIP@
VERSION='@VERSION@'
KIND='@KIND@'

die() { echo "CubeCheck: $*" >&2; exit 1; }

SELF=$0
case "$SELF" in
  /*) ;;
  *) SELF=$(pwd)/$SELF ;;
esac
HERE=$(CDPATH= cd -- "$(dirname -- "$SELF")" && pwd)

if [ -n "${CUBECHECK_HOME:-}" ]; then
  DEST=$CUBECHECK_HOME
elif [ -w "$HERE" ]; then
  DEST="$HERE/cubecheck-payload"
else
  DEST="${HOME}/.local/opt/cubecheck/${VERSION}-${KIND}"
fi

STAMP="$DEST/.cubecheck-stamp"
NEED=1
if [ -f "$STAMP" ] && [ "$(cat "$STAMP" 2>/dev/null || true)" = "${VERSION}-${KIND}" ]; then
  if [ -f "$DEST/cubecheck" ] || [ -f "$DEST/payload/linux-x64/cubecheck" ] || [ -f "$DEST/payload/linux-x86/cubecheck" ]; then
    NEED=0
  fi
fi

if [ "$NEED" -eq 1 ]; then
  mkdir -p "$DEST"
  if command -v tar >/dev/null 2>&1; then
    tail -n +"$SKIP" "$SELF" | tar -xz -C "$DEST"
  else
    die "need tar to extract this installer"
  fi
  printf '%s\n' "${VERSION}-${KIND}" > "$STAMP"
fi

BIN=""
if [ -f "$DEST/cubecheck" ]; then
  BIN="$DEST/cubecheck"
else
  arch=$(uname -m 2>/dev/null || echo unknown)
  case "$arch" in
    x86_64|amd64) try="linux-x64 linux-x86" ;;
    i386|i486|i586|i686|x86) try="linux-x86 linux-x64" ;;
    *) try="linux-x64 linux-x86" ;;
  esac
  for id in $try; do
    if [ -f "$DEST/payload/$id/cubecheck" ]; then
      BIN="$DEST/payload/$id/cubecheck"
      break
    fi
  done
fi

[ -n "$BIN" ] || die "payload has no cubecheck ELF (extract dir: $DEST)"
chmod +x "$BIN" 2>/dev/null || true
export CUBECHECK_PORTABLE=1
cd "$(dirname -- "$BIN")"
exec "$BIN" "$@"
