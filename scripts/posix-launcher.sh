#!/bin/sh
# POSIX CubeCheck launcher for linux-universal / universal bundles.
# Picks payload/<os-arch>/cubecheck and execs it.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PAYLOAD="$ROOT/payload"

die() {
  echo "CubeCheck: $*" >&2
  echo "CubeCheck" > "$ROOT/CubeCheck-error.txt"
  echo >> "$ROOT/CubeCheck-error.txt"
  echo "$*" >> "$ROOT/CubeCheck-error.txt"
  exit 1
}

offline=0
if [ -f "$ROOT/.offline" ] || [ -f "$ROOT/assets/.offline" ]; then
  offline=1
fi

os=$(uname -s 2>/dev/null || echo unknown)
arch=$(uname -m 2>/dev/null || echo unknown)

candidates=""
case "$os" in
  Linux)
    case "$arch" in
      x86_64|amd64) candidates="linux-x64 linux-x86" ;;
      i386|i486|i586|i686|x86) candidates="linux-x86 linux-x64" ;;
      *) candidates="linux-x64 linux-x86" ;;
    esac
    ;;
  Darwin)
    candidates="macos-universal macos-arm64 macos-x64"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    case "$arch" in
      x86_64|amd64) candidates="windows-x64 windows-x86" ;;
      *) candidates="windows-x86 windows-x64" ;;
    esac
    ;;
  *)
    die "Неизвестная ОС: $os/$arch"
    ;;
esac

pick=""
pick_dir=""
for id in $candidates; do
  dir="$PAYLOAD/$id"
  if [ -f "$dir/cubecheck" ]; then
    pick="$dir/cubecheck"
    pick_dir="$dir"
    break
  fi
  if [ -f "$dir/cubecheck.exe" ]; then
    pick="$dir/cubecheck.exe"
    pick_dir="$dir"
    break
  fi
done

if [ -z "$pick" ]; then
  listed=$(ls "$PAYLOAD" 2>/dev/null || echo "(пусто)")
  die "Нет сборки CubeCheck для $os/$arch.
Искали: $candidates
В payload/: $listed
Соберите нужный артефакт (build.sh / build.bat) или запустите с ОС, для которой есть payload."
fi

if [ -f "$pick_dir/.offline" ] || [ -f "$pick_dir/assets/.offline" ]; then
  offline=1
fi

export CUBECHECK_PORTABLE=1
export CUBECHECK_LAUNCHER_OS="$os"
if [ "$offline" -eq 1 ]; then
  export CUBECHECK_OFFLINE=1
fi

cd "$pick_dir"
if [ -x "$pick" ]; then
  exec "$pick" "$@"
fi
# Windows checkout / zip may drop +x
chmod +x "$pick" 2>/dev/null || true
exec "$pick" "$@"
