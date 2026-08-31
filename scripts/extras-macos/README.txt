CubeCheck macOS extras (offline)
===============================

Авторы: AuraStudio, AnProject
Windows-утилиты сюда не входят. Качать ничего не нужно.

Встроенные программы macOS (уже на компьютере):

  Spotlight (mdfind)   — аналог Everything
  Недавние файлы       — аналог Shellbag
  Activity Monitor     — аналог System Informer / Process Explorer
  fs_usage             — аналог Process Monitor
  Login Items          — аналог Autoruns

В payload/*/assets/bin дополнительно лежат официальные Darwin-бинарники
(fd, rg, fzf, lf, procs, btm), если сборка их скачала.

Скрипт extras/install-macos-tools.sh не является продуктом — только
необязательный Homebrew, если хотите системные пакеты.

Если в zip нет payload/osx-x64/cubecheck или payload/osx-arm64/cubecheck,
бинарник CubeCheck нужно собрать на Mac (на Windows нет Apple SDK):

  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build -p cubecheck --release --bin cubecheck --features gui --target aarch64-apple-darwin
