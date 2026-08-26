#!/usr/bin/env bash
# Pack a Debian binary package from a prepared tree:
#   scripts/pack-deb.sh <pkgroot> <output.deb>
# pkgroot must contain DEBIAN/control and the installed files (usr/...).
set -eu

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <pkgroot> <output.deb>" >&2
  exit 2
fi

PKGROOT=$(CDPATH= cd -- "$1" && pwd)
OUT=$2

if [ ! -f "$PKGROOT/DEBIAN/control" ]; then
  echo "missing $PKGROOT/DEBIAN/control" >&2
  exit 1
fi

BIN="$PKGROOT/usr/bin/cubecheck"
if [ ! -f "$BIN" ]; then
  echo "missing $BIN — Linux binary was not built" >&2
  exit 1
fi
MAG=$(dd if="$BIN" bs=4 count=1 2>/dev/null || true)
if [ "$MAG" != $'\x7fELF' ]; then
  echo "$BIN is not a real ELF — refusing to pack a dummy .deb" >&2
  exit 1
fi

chmod 755 "$PKGROOT/usr/bin/cubecheck" 2>/dev/null || true
mkdir -p "$(dirname "$OUT")"

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build "$PKGROOT" "$OUT"
  echo "wrote $OUT"
  exit 0
fi

# Fallback: GNU ar + tar (valid .deb)
WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT
printf '2.0\n' > "$WORKDIR/debian-binary"

(
  cd "$PKGROOT/DEBIAN"
  tar --format=ustar -czf "$WORKDIR/control.tar.gz" .
)
(
  cd "$PKGROOT"
  tar --format=ustar -czf "$WORKDIR/data.tar.gz" --exclude=DEBIAN usr
)

if command -v ar >/dev/null 2>&1; then
  rm -f "$OUT"
  ar r "$OUT" "$WORKDIR/debian-binary" "$WORKDIR/control.tar.gz" "$WORKDIR/data.tar.gz"
  echo "wrote $OUT (ar)"
  exit 0
fi

echo "Need dpkg-deb or ar to produce a .deb. Tree is ready at $PKGROOT" >&2
exit 1
