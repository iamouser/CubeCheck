#!/bin/sh
# CubeCheck POSIX launcher: picks payload/<os-arch>/cubecheck
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PAYLOAD="$ROOT/payload"

die() {
  echo "CubeCheck: $*" >&2
  {
    echo "CubeCheck"
    echo
    echo "$*"
  } > "$ROOT/CubeCheck-error.txt"
  exit 1
}

uname_s=$(uname -s 2>/dev/null || echo unknown)
uname_m=$(uname -m 2>/dev/null || echo unknown)

candidates=""
case "$uname_s" in
  Linux)
    case "$uname_m" in
      x86_64|amd64) candidates="linux-x64 linux-x86" ;;
      i386|i686)    candidates="linux-x86 linux-x64" ;;
      aarch64|arm64) candidates="linux-arm64 linux-x64" ;;
      *)            candidates="linux-x64 linux-x86" ;;
    esac
    ;;
  Darwin)
    case "$uname_m" in
      arm64) candidates="osx-arm64 osx-x64 macos-arm64 macos-x64" ;;
      *)     candidates="osx-x64 osx-arm64 macos-x64 macos-arm64" ;;
    esac
    ;;
  *)
    die "Эта сборка для Linux и macOS. На Windows запустите cubecheck-launcher.exe"
    ;;
esac

exe=""
kind=""
dir=""
tried=""
for k in $candidates; do
  d="$PAYLOAD/$k"
  if [ -x "$d/cubecheck" ]; then
    exe="$d/cubecheck"
    kind="$k"
    dir="$d"
    break
  fi
  if [ -f "$d/cubecheck" ]; then
    exe="$d/cubecheck"
    kind="$k"
    dir="$d"
    break
  fi
  tried="$tried  $k ($d)
"
done

if [ -z "$exe" ]; then
  die "Нет сборки CubeCheck для этой ОС.
Искали:
$tried"
fi

chmod +x "$exe" 2>/dev/null || true
chmod +x "$dir/assets/bin/"* "$dir/extras/bin/"* "$ROOT/extras/bin/"* 2>/dev/null || true
export CUBECHECK_PORTABLE=1
export CUBECHECK_LAUNCHER_OS="$kind"
export APPIMAGE_EXTRACT_AND_RUN=1
if [ -f "$ROOT/.offline" ] || [ -f "$dir/.offline" ] || [ -f "$dir/assets/.offline" ]; then
  export CUBECHECK_OFFLINE=1
fi
export PATH="$dir/assets/bin:$dir/extras/bin:$ROOT/extras/bin:$PATH"

cd "$dir"
exec "$exe" "$@"
