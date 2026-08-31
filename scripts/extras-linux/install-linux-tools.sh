#!/bin/sh
# Optional. Offline pack already contains fd/rg/fzf/lf/btop/btm/procs/busybox
# (and Mission Center AppImage on x86_64) in assets/bin. Do not treat this
# script as the way to get tools.
set -eu

say() { printf '%s\n' "$*"; }

say "CubeCheck offline: утилиты уже в assets/bin. Этот скрипт только ставит"
say "необязательные GTK-пакеты дистрибутива (FSearch и т.п.)."

if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y fsearch catfish plocate htop gnome-system-monitor || true
elif command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y fsearch catfish plocate htop gnome-system-monitor || true
elif command -v pacman >/dev/null 2>&1; then
  sudo pacman -S --needed --noconfirm fsearch catfish plocate htop gnome-system-monitor || true
else
  say "Пакетный менеджер не найден. Это не ошибка: bundled-инструменты уже в assets/bin."
fi

say "Готово. Запуск: ./cubecheck.sh"
