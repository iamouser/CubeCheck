#!/usr/bin/env bash
# Pack a self-extracting CubeCheck .sh from a staging directory.
#   scripts/pack-linux-sh.sh <stagedir> <output.sh> <kind> [version]
#
# stagedir must contain a real ELF named cubecheck, or payload/linux-x64|x86/cubecheck.
set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <stagedir> <output.sh> <kind> [version]" >&2
  exit 2
fi

STAGE=$(CDPATH= cd -- "$1" && pwd)
OUT=$2
KIND=$3
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
VERSION=${4:-}
if [ -z "$VERSION" ]; then
  VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)
fi

TEMPLATE="$ROOT/scripts/linux-sh-header.sh"
[ -f "$TEMPLATE" ] || { echo "missing $TEMPLATE" >&2; exit 1; }

is_elf() {
  [ -f "$1" ] || return 1
  local mag
  mag=$(dd if="$1" bs=4 count=1 2>/dev/null || true)
  [ "$mag" = $'\x7fELF' ]
}

have=0
if is_elf "$STAGE/cubecheck"; then
  have=1
fi
for id in linux-x64 linux-x86; do
  if is_elf "$STAGE/payload/$id/cubecheck"; then
    have=1
  fi
done
[ "$have" -eq 1 ] || { echo "no ELF cubecheck in $STAGE — refusing dummy .sh" >&2; exit 1; }

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT
TAR="$WORKDIR/payload.tar.gz"
tar -C "$STAGE" -czf "$TAR" .

# First pass: SKIP=0 (same line count after we substitute a number)
header=$WORKDIR/header.sh
sed -e "s/@VERSION@/$VERSION/g" -e "s/@KIND@/$KIND/g" -e "s/@SKIP@/0/" "$TEMPLATE" | tr -d '\r' > "$header"
# ensure trailing newline
[ -z "$(tail -c1 "$header")" ] || printf '\n' >> "$header"
SKIP=$(($(wc -l < "$header") + 1))
sed -e "s/@VERSION@/$VERSION/g" -e "s/@KIND@/$KIND/g" -e "s/@SKIP@/$SKIP/" "$TEMPLATE" | tr -d '\r' > "$header"
[ -z "$(tail -c1 "$header")" ] || printf '\n' >> "$header"
SKIP=$(($(wc -l < "$header") + 1))
# SKIP may have changed digit width; rewrite once more so tail -n +SKIP is exact
sed -e "s/@VERSION@/$VERSION/g" -e "s/@KIND@/$KIND/g" -e "s/@SKIP@/$SKIP/" "$TEMPLATE" | tr -d '\r' > "$header"
[ -z "$(tail -c1 "$header")" ] || printf '\n' >> "$header"
SKIP=$(($(wc -l < "$header") + 1))
sed -e "s/@VERSION@/$VERSION/g" -e "s/@KIND@/$KIND/g" -e "s/@SKIP@/$SKIP/" "$TEMPLATE" | tr -d '\r' > "$header"
[ -z "$(tail -c1 "$header")" ] || printf '\n' >> "$header"

mkdir -p "$(dirname -- "$OUT")"
cat "$header" "$TAR" > "$OUT"
chmod +x "$OUT" 2>/dev/null || true
echo "wrote $OUT"
