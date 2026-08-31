#!/bin/sh
# CubeCheck 1.1 beta — установщик macOS (те же шаги, что и мастер).
# Mach-O cubecheck / CubeCheck.app могут отсутствовать, если пакет собран на Windows.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PRODUCT=CubeCheck
VERSION="1.1 beta"
AUTHORS="AuraStudio, AnProject"
PAYLOAD_URL="${CUBECHECK_PAYLOAD_URL:-https://github.com/jumpworlds/CubeCheck-payload/archive/refs/heads/main.zip}"

die() { echo "CubeCheck: $*" >&2; exit 1; }

is_offline() {
  case "${CUBECHECK_OFFLINE:-}" in
    ''|0|false|no|NO) ;;
    *) return 0 ;;
  esac
  [ -f "$ROOT/.offline" ] && return 0
  [ -f "$ROOT/assets/.offline" ] && return 0
  case "$(basename "${0:-}")" in
    *offline*) return 0 ;;
  esac
  case "$(basename "${CUBECHECK_SETUP_RUN:-}")" in
    *offline*) return 0 ;;
  esac
  return 1
}

if [ "$(uname -s)" != "Darwin" ]; then
  die "Этот скрипт для macOS. На Linux запустите install-linux.sh"
fi

arch=$(uname -m 2>/dev/null || echo unknown)
case "$arch" in
  arm64) KEY=osx-arm64; ALT=osx-x64 ;;
  *)     KEY=osx-x64; ALT=osx-arm64 ;;
esac

LICENSE=""
for c in "$ROOT/LICENSE.md" "$ROOT/../LICENSE.md"; do
  if [ -f "$c" ]; then LICENSE=$c; break; fi
done
[ -n "$LICENSE" ] || die "Нет LICENSE.md"

echo "CubeCheck $VERSION — $AUTHORS"
if is_offline; then
  echo "Режим: офлайн (загрузка отключена)"
else
  echo "Режим: онлайн"
fi
echo
echo "=== 1. Лицензия (MIT) ==="
cat "$LICENSE"
echo
printf "Принять лицензию? [нет/да]: "
read -r accept
case "$accept" in
  да|Да|yes|YES|y|Y) ;;
  *) die "Нужно принять лицензию, чтобы продолжить." ;;
esac

echo
echo "=== 2. Ярлыки ==="
printf "Ярлык на рабочем столе? [да/нет]: "
read -r desk
printf "Псевдоним в Applications? [да/нет]: "
read -r menu
case "${desk:-да}" in нет|Нет|no|NO|n|N) DESK=0 ;; *) DESK=1 ;; esac
case "${menu:-да}" in нет|Нет|no|NO|n|N) MENU=0 ;; *) MENU=1 ;; esac

DEF=/Applications/CubeCheck
echo
echo "=== 3. Папка установки ==="
printf "Каталог [%s]: " "$DEF"
read -r dest
dest=${dest:-$DEF}
printf "Запустить после установки? [да/нет]: "
read -r launch
case "${launch:-да}" in нет|Нет|no|NO|n|N) LAUNCH=0 ;; *) LAUNCH=1 ;; esac

pick() {
  base=$3
  for k in "$1" "$2"; do
    if [ -d "$base/$k" ] && [ -f "$base/$k/cubecheck" ]; then echo "$base/$k"; return; fi
    if [ -d "$base/payload/$k" ] && [ -f "$base/payload/$k/cubecheck" ]; then echo "$base/payload/$k"; return; fi
  done
  if [ -f "$base/cubecheck" ]; then echo "$base"; return; fi
  if [ -d "$base/CubeCheck.app" ]; then echo "$base"; return; fi
  echo ""
}

unwrap_github() {
  ex=$1
  if [ -d "$ex/CubeCheck-payload-main" ]; then echo "$ex/CubeCheck-payload-main"; return; fi
  n=$(find "$ex" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
  f=$(find "$ex" -mindepth 1 -maxdepth 1 -type f | wc -l | tr -d ' ')
  if [ "$n" = "1" ] && [ "$f" = "0" ]; then
    find "$ex" -mindepth 1 -maxdepth 1 -type d
    return
  fi
  echo "$ex"
}

extract_zip() {
  z=$1
  d=$2
  mkdir -p "$d"
  if command -v unzip >/dev/null 2>&1; then
    unzip -qo "$z" -d "$d"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$z" "$d"
  else
    die "Нужен unzip или python3, чтобы распаковать payload"
  fi
}

find_extras() {
  for d in "$ROOT/extras/bin" "$ROOT/extras/$KEY" "$ROOT/payload/$KEY/assets/bin" "$ROOT/assets/bin"; do
    if [ -d "$d" ] && [ "$(find "$d" -type f 2>/dev/null | wc -l | tr -d ' ')" != "0" ]; then
      echo "$d"
      return
    fi
  done
  echo ""
}

TMP_DL=""
cleanup() {
  if [ -n "$TMP_DL" ] && [ -d "$TMP_DL" ]; then rm -rf "$TMP_DL" || true; fi
}
trap cleanup EXIT INT TERM

SRC=$(pick "$KEY" "$ALT" "$ROOT")
if [ -z "$SRC" ] && ! is_offline; then
  case "$PAYLOAD_URL" in
    https://*) ;;
    *) die "Разрешена только загрузка по HTTPS." ;;
  esac
  echo
  echo "Загрузка $PAYLOAD_URL …"
  TMP_DL=$(mktemp -d "${TMPDIR:-/tmp}/cubecheck-dl.XXXXXX")
  zip="$TMP_DL/payload.zip"
  if command -v curl >/dev/null 2>&1; then
    curl -L --fail --retry 3 -A "CubeCheck-Setup/1.1-beta" -o "$zip" "$PAYLOAD_URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$zip" "$PAYLOAD_URL"
  else
    die "Нужен curl или wget для загрузки payload"
  fi
  extract_zip "$zip" "$TMP_DL/ex"
  unpacked=$(unwrap_github "$TMP_DL/ex")
  SRC=$(pick "$KEY" "$ALT" "$unpacked")
fi

EXTRAS=$(find_extras)
HAVE_APP=0
if [ -n "$SRC" ] && { [ -f "$SRC/cubecheck" ] || [ -d "$SRC/CubeCheck.app" ]; }; then
  HAVE_APP=1
fi

if [ "$HAVE_APP" -eq 0 ]; then
  echo
  echo "В этом пакете нет Mach-O cubecheck и нет CubeCheck.app."
  echo "Пакет собран на Windows без Apple SDK — GUI CubeCheck сюда не входит."
  if [ -f "$ROOT/README-macos.txt" ]; then
    echo
    cat "$ROOT/README-macos.txt"
    echo
  fi
  if [ -z "$EXTRAS" ]; then
    if is_offline; then
      die "Офлайн-установщик: нет CubeCheck.app / cubecheck и нет portable-утилит. Загрузка отключена."
    fi
    die "Нет macOS payload (osx-arm64 / osx-x64) с бинарником cubecheck"
  fi
  echo "Установлю только portable-утилиты в $dest/assets/bin (без CubeCheck.app)."
fi

echo
echo "=== 4. Установка ==="
mkdir -p "$dest/assets/bin" "$dest/reports"
if [ "$HAVE_APP" -eq 1 ]; then
  echo "Копирование CubeCheck в $dest …"
  cp -a "$SRC/." "$dest/"
  chmod +x "$dest/cubecheck" "$dest/cubecheck.sh" 2>/dev/null || true
fi
if [ -n "$EXTRAS" ]; then
  echo "Копирование portable-утилит …"
  mkdir -p "$dest/assets/bin"
  cp -R "$EXTRAS/." "$dest/assets/bin/" 2>/dev/null || true
  chmod +x "$dest/assets/bin/"* 2>/dev/null || true
fi
if [ -f "$ROOT/README-macos.txt" ]; then
  cp "$ROOT/README-macos.txt" "$dest/README-macos.txt"
fi
if [ -f "$LICENSE" ]; then
  cp "$LICENSE" "$dest/LICENSE.md" 2>/dev/null || true
fi
if [ ! -f "$dest/settings.json" ] && [ -f "$dest/assets/settings.default.json" ]; then
  cp "$dest/assets/settings.default.json" "$dest/settings.json"
fi
if is_offline; then
  : > "$dest/.offline"
  : > "$dest/assets/.offline"
fi

BIN=""
if [ -x "$dest/cubecheck" ]; then BIN="$dest/cubecheck"; fi
if [ -z "$BIN" ] && [ -x "$dest/cubecheck.sh" ]; then BIN="$dest/cubecheck.sh"; fi

if [ "$HAVE_APP" -eq 1 ] && [ "$MENU" -eq 1 ]; then
  if [ "$(cd "$dest" && pwd)" != "/Applications/CubeCheck" ]; then
    ln -sfn "$dest" /Applications/CubeCheck 2>/dev/null || true
  fi
fi
if [ "$HAVE_APP" -eq 1 ] && [ "$DESK" -eq 1 ]; then
  ln -sfn "$dest" "$HOME/Desktop/CubeCheck" 2>/dev/null || true
fi

echo
echo "=== 5. Установка завершена ==="
echo "CubeCheck $VERSION — $AUTHORS"
if [ "$HAVE_APP" -eq 0 ]; then
  echo "CubeCheck.app в этом пакете нет. См. README-macos.txt"
fi
if [ "$LAUNCH" -eq 1 ] && [ -n "$BIN" ]; then
  (cd "$dest" && exec "$BIN") || true
fi
