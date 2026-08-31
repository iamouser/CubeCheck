#!/bin/sh
# Optional. Spotlight, Activity Monitor, fs_usage, Login Items are built-in.
# Portable fd/rg/fzf/lf/procs/btm are already in assets/bin when fetched.
set -eu

say() { printf '%s\n' "$*"; }

say "Встроено в macOS (офлайн, качать не нужно):"
say "  Spotlight / mdfind, Finder, Activity Monitor, fs_usage, Login Items"
say "В assets/bin уже могут быть официальные fd/rg/fzf/lf/procs/btm."

if command -v brew >/dev/null 2>&1; then
  say "Необязательно (Homebrew): fd ripgrep fzf bottom"
  brew install fd ripgrep fzf bottom || true
else
  say "Homebrew не нужен для офлайн-пакета."
fi

say "Готово. Запуск: ./cubecheck.sh"
