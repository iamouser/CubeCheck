#!/bin/sh
# CubeCheck 1.1 beta — установщик Linux (те же шаги, что и мастер).
# Онлайн: локальный payload или загрузка GitHub zip.
# Офлайн: только локальные файлы, без HTTP.
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

if [ "$(uname -s)" != "Linux" ]; then
  die "Этот скрипт для Linux. На macOS запустите install-macos.sh"
fi

arch=$(uname -m 2>/dev/null || echo unknown)
case "$arch" in
  x86_64|amd64) KEY=linux-x64 ;;
  i386|i686)    KEY=linux-x86 ;;
  *)            KEY=linux-x64 ;;
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
printf "Файл .desktop в меню приложений? [да/нет]: "
read -r menu
case "${desk:-да}" in нет|Нет|no|NO|n|N) DESK=0 ;; *) DESK=1 ;; esac
case "${menu:-да}" in нет|Нет|no|NO|n|N) MENU=0 ;; *) MENU=1 ;; esac

if [ "$(id -u)" -eq 0 ] || [ -w /opt ] || [ -w /opt/CubeCheck ] 2>/dev/null; then
  DEF=/opt/CubeCheck
else
  DEF="${XDG_DATA_HOME:-$HOME/.local/share}/CubeCheck"
fi

echo
echo "=== 3. Папка установки ==="
printf "Каталог [%s]: " "$DEF"
read -r dest
dest=${dest:-$DEF}
printf "Запустить после установки? [да/нет]: "
read -r launch
case "${launch:-да}" in нет|Нет|no|NO|n|N) LAUNCH=0 ;; *) LAUNCH=1 ;; esac

find_src() {
  base=$1
  if [ -d "$base/$KEY" ] && [ -f "$base/$KEY/cubecheck" ]; then echo "$base/$KEY"; return; fi
  if [ -d "$base/payload/$KEY" ] && [ -f "$base/payload/$KEY/cubecheck" ]; then echo "$base/payload/$KEY"; return; fi
  if [ -f "$base/cubecheck" ]; then echo "$base"; return; fi
  echo ""
}

unwrap_github() {
  ex=$1
  if [ -d "$ex/CubeCheck-payload-main" ]; then echo "$ex/CubeCheck-payload-main"; return; fi
  n=$(find "$ex" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
  f=$(find "$ex" -mindepth 1 -maxdepth 1 -type f | wc -l | tr -d ' ')
  if [ "$n" = "1" ] && [ "$f" = "0" ]; then
    d=$(find "$ex" -mindepth 1 -maxdepth 1 -type d)
    echo "$d"
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

TMP_DL=""
cleanup() {
  if [ -n "$TMP_DL" ] && [ -d "$TMP_DL" ]; then rm -rf "$TMP_DL" || true; fi
}
trap cleanup EXIT INT TERM

SRC=$(find_src "$ROOT")
if [ -z "$SRC" ]; then
  if is_offline; then
    die "Офлайн-установщик: нет локального payload $KEY. Загрузка из сети отключена."
  fi
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
  SRC=$(find_src "$unpacked")
  [ -n "$SRC" ] || die "В загруженном архиве нет payload $KEY (ELF cubecheck)"
fi

echo
echo "=== 4. Установка ==="
mkdir -p "$dest"
echo "Копирование в $dest …"
cp -a "$SRC/." "$dest/"
chmod +x "$dest/cubecheck" "$dest/cubecheck.sh" "$dest/assets/bin/"* 2>/dev/null || true
mkdir -p "$dest/reports"
if [ ! -f "$dest/settings.json" ] && [ -f "$dest/assets/settings.default.json" ]; then
  cp "$dest/assets/settings.default.json" "$dest/settings.json"
fi
if is_offline; then
  : > "$dest/.offline"
  mkdir -p "$dest/assets"
  : > "$dest/assets/.offline"
fi

ICON="$dest/assets/cubecheck.ico"
BIN="$dest/cubecheck.sh"
[ -x "$dest/cubecheck" ] && BIN="$dest/cubecheck"
DESKTOP_BODY="[Desktop Entry]
Type=Application
Name=CubeCheck
Comment=CubeCheck $VERSION
Exec=\"$BIN\"
Path=$dest
Icon=${ICON}
Terminal=false
Categories=Utility;
"

if [ "$MENU" -eq 1 ]; then
  apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
  mkdir -p "$apps"
  printf '%s\n' "$DESKTOP_BODY" > "$apps/cubecheck.desktop"
  chmod +x "$apps/cubecheck.desktop" 2>/dev/null || true
fi
if [ "$DESK" -eq 1 ]; then
  deskdir="${XDG_DESKTOP_DIR:-$HOME/Desktop}"
  mkdir -p "$deskdir"
  printf '%s\n' "$DESKTOP_BODY" > "$deskdir/CubeCheck.desktop"
  chmod +x "$deskdir/CubeCheck.desktop" 2>/dev/null || true
fi

echo
echo "=== 5. Установка завершена ==="
echo "CubeCheck $VERSION — $AUTHORS"
if [ "$LAUNCH" -eq 1 ]; then
  (cd "$dest" && exec "$BIN") || true
fi
