# CubeCheck

**CubeCheck** — программа, которая помогает проверить компьютер на читы Minecraft. Она ищет известные имена читов в процессах, файлах, автозагрузке и логах, сохраняет отчёт и запускает утилиты проверки.

Авторы: **[AuraStudio](https://telegram.me/cubecheck)**, **[AnProject](https://discord.gg/Dwqu8xmaEc)**.

## Возможности

- Автопроверка: процессы, файлы (рабочий стол, загрузки, `.minecraft`), автозагрузка, логи Minecraft.
- Сохранение отчёта (YAML) в папку `reports`.
- Очистка логов Minecraft (`%APPDATA%\.minecraft\logs` / `~/.minecraft/logs` / macOS Application Support).
- Запуск утилит проверки (набор зависит от ОС — см. таблицу ниже).
- Раздел «Компоненты»: на Windows — загрузка по HTTPS с проверкой издателя (Authenticode); на Linux/macOS — подсказки пакетного менеджера, без скачивания чужих `.exe`.
- Сведения о системе, корзина, ссылка на HolyCheck (HolyWorld).
- Темы оформления, свечение интерфейса, масштаб, автосохранение настроек.
- Тонкий установщик `cubecheck-setup`: копирует программу в `C:\Program Files\CubeCheck` и создаёт ярлыки. Утилиты скачиваются при первом запуске, в установщик они не входят (кроме сборки **universal-local**).

## Требования

- Обычный запуск GUI: Windows 10/11, 64-bit. Сборка `windows-x86` — 32-bit Windows / WoW64.
- Linux: GTK-поиск (FSearch или Catfish) ставится из репозитория; автопроверка может использовать `plocate`. macOS: Spotlight, Activity Monitor, `fs_usage` — встроены.
- Лаунчер **universal** рассчитан на Windows 7/10/11, Linux и macOS *если в комплекте есть соответствующий payload*.
- Для сборки из исходников: [Rust](https://rustup.rs/) (стабильный канал, edition 2021). Проверено с `rustc` 1.93.

## Сборка

### Быстро (как раньше)

```bat
build.bat
```

Или `START.bat`. После сборки в корне появятся `cubecheck.exe` и `CubeCheck-Setup.exe` (они в `.gitignore`). Дубликат лежит в `dist/windows-x64/`.

Отладочный запуск:

```bat
cargo run --release --bin cubecheck
```

### Артефакты (`dist/` и `build/`)

| Команда | Результат |
|---------|-----------|
| `build.bat` / `build.bat windows-x64` | `dist/windows-x64/cubecheck-windows-x64.exe`, `CubeCheck-Setup.exe` |
| `build.bat windows-x86` | `dist/windows-x86/cubecheck-windows-x86.exe` (`i686-pc-windows-msvc`) |
| `build.bat linux-deb-x64` | ELF + `CubeCheck-<ver>-linux-x64.sh`; `.deb` только если `usr/bin/cubecheck` — настоящий ELF |
| `build.bat linux-deb-x86` | то же для i686 / `linux-x86.sh` |
| `build.bat linux-universal` | `CubeCheck-<ver>-linux-universal.sh` (оба ELF внутри), если хотя бы один ELF собран |
| `build.bat macos-universal` | zip с Mach-O `cubecheck` (без расширения); lipo arm64+x64 если есть SDK/`lipo` |
| `build.bat universal` | `dist/universal/CubeCheck-universal/` — лаунчер + payload по ОС |
| `build.bat universal-local` | то же + вендорные утилиты, **без загрузок из сети** |
| `build.bat all` | по очереди все цели; Linux/macOS на Windows-ПК могут быть FAIL с понятной ошибкой |
| `build.bat publish` | только пересобрать `build/` из уже готового `dist/` (без компиляции) |

На Linux/macOS: `./build.sh <цель>` (тот же набор имён).

### GitHub Releases

После `build.bat all` в папке `build/` лежит плоский набор файлов для вкладки **Releases**. Загружайте **файлы из `build/`**, не деревья из `dist/` и не заглушки.

**Linux:** скачайте `CubeCheck-<версия>-linux-x64.sh` (или `linux-x86` / `linux-universal`). Это self-extracting installer, внутри настоящий ELF `cubecheck` без расширения:

```bash
chmod +x CubeCheck-1.0.0-beta-linux-x64.sh
./CubeCheck-1.0.0-beta-linux-x64.sh
```

Скрипт распаковывает payload рядом с собой (`cubecheck-payload/`) или в `~/.local/opt/cubecheck` и запускает ELF. `.deb` попадает в `build/` только если `usr/bin/cubecheck` — реальный ELF (магия `\x7fELF`). Фейковый tarball/README **не** публикуется.

**macOS:** `CubeCheck-<версия>-macos-universal.zip` — внутри Mach-O с именем `cubecheck` (без `.exe` / без `.app`). При необходимости рядом лежат срезы `cubecheck-arm64` и `cubecheck-x64`.

```bash
unzip CubeCheck-1.0.0-beta-macos-universal.zip
chmod +x CubeCheck/cubecheck
./CubeCheck/cubecheck
```

Кросс-сборка Linux с Windows: поставьте [zig](https://ziglang.org/) (`zig cc` для `zstd-sys`) и `rustup target add x86_64-unknown-linux-gnu`. TLS на всех ОС — **rustls**, без OpenSSL. Без ELF `build.bat linux-deb-x64` **завершится с ошибкой** (см. `dist/linux-deb-x64/compile.log`); каркас `.deb` останется в `dist/`, в `build/` `.sh` не появится.

macOS Mach-O с Windows без Apple SDK (libSystem / AppKit) **не подделывается**. Рецепт: `dist/macos-universal/README.txt` и `./build.sh macos-universal` на Mac (`cargo build --release --target aarch64-apple-darwin`). Не скачивайте Xcode SDK с случайных зеркал.

## universal и universal-local

Один `.exe` не может одновременно быть PE, ELF и Mach-O. **universal** — переносной комплект:

```
CubeCheck-universal/
  cubecheck.exe          лаунчер Windows (std, Win7+)
  cubecheck.sh           лаунчер Linux/macOS
  cubecheck.command      двойной щелчок в Finder
  payload/
    windows-x64/
    windows-x86/
    linux-x64/
    linux-x86/
    macos-universal/
```

Лаунчер смотрит на ОС/архитектуру (`PROCESSOR_ARCHITECTURE` / `uname`) и запускает подходящий бинарник из `payload/`. Если его нет — пишет ошибку в консоль и `CubeCheck-error.txt`.

- **universal** — обычное приложение: на Windows утилиты по-прежнему можно скачать в «Компонентах».
- **universal-local** — офлайн. Сеть для утилит **не используется**. Файлы Everything / Sysinternals / System Informer / Shellbag копируются из локальной `assets/` в payload Windows на этапе сборки. Если какого-то файла нет, сборка **падает** со списком недостающего (тишина и «пустой офлайн-пак» запрещены).

Офлайн включается так (достаточно одного):

- Cargo-фича `offline` (`cargo build --release --bin cubecheck --features offline`)
- переменная окружения `CUBECHECK_OFFLINE=1` (лаунчер выставляет её сам, если рядом есть файл `.offline`)
- файл `.offline` рядом с exe или в `assets/`

Портативный режим (настройки и `assets` рядом с exe, без `C:\Program Files\CubeCheck`): `CUBECHECK_PORTABLE=1` или файл `.portable`. Сборки universal всегда портативные. На Linux/macOS установка в Program Files не используется.

Сторонние Windows-`.exe` на Linux/macOS **не запускаются**. Вместо них — системные и FOSS-аналоги (таблица ниже). Сборка **universal-local** по-прежнему вендорит только Windows-утилиты в `payload/windows-*/assets/`; Activity Monitor и Spotlight не пакуются.

## Утилиты проверки по ОС

Один и тот же сценарий чекера: поиск имён на диске, процессы, автозагрузка, недавние файлы, корзина, при возможности — живая файловая активность.

**Shellbags, реестр Windows и PE не имеют идентичного POSIX-клона.** Замены закрывают те же *задачи* проверки, а не копируют формат артефактов Windows.

| Задача чекера | Windows | Linux | macOS |
|---------------|---------|-------|-------|
| Поиск имён на диске | [Everything](https://www.voidtools.com/) (`-search`, OR через `\|`) | [FSearch](https://github.com/cboxdoerfer/fsearch) (GUI, синтаксис `OR`/`\|\|`) + [Catfish](https://docs.xfce.org/apps/catfish/start); автопроверка кормит запрос в **plocate**/`locate` (`regex` `a\|b\|c`), потому что у FSearch нет CLI-поиска | Spotlight через **`mdfind`** (`kMDItemDisplayName … \|\| …`) |
| Недавние папки/файлы | [Shellbag Analyzer](https://privazer.com/) | Встроенный список `~/.local/share/recently-used.xbel` | Встроенный список через `mdfind` (`kMDItemLastUsedDate`) |
| Процессы | [System Informer](https://github.com/winsiderss/systeminformer) | [Mission Center](https://missioncenter.io/) (`missioncenter` / Flatpak), иначе GNOME System Monitor | **Activity Monitor** (`open -a "Activity Monitor"`) |
| Живая активность | [Process Monitor](https://learn.microsoft.com/sysinternals/downloads/procmon) | **sysdig** / `csysdig`, запасной вариант `lsof -r` | встроенный **`fs_usage`** в Terminal (или Console.app) |
| Автозагрузка | [Autoruns](https://learn.microsoft.com/sysinternals/downloads/autoruns) | Встроенный список: `~/.config/autostart`, `/etc/xdg/autostart`, `systemctl --user` / system `enabled` | **Login Items** (System Settings) + список LaunchAgents/Daemons |
| Дерево процессов | [Process Explorer](https://learn.microsoft.com/sysinternals/downloads/process-explorer) | GNOME System Monitor / `htop` | Activity Monitor |
| Корзина | Корзина Windows | XDG Trash (`gio open trash:///` / `xdg-open`) | `open ~/.Trash` |

Автопроверка на Windows открывает Everything с запросом `(имя1 \| имя2 \| …)`. На Linux/macOS она запускает **тот же список имён** в синтаксисе FSearch/`plocate` или Spotlight, а не `Everything.exe`.

Реестр Run/RunOnce сканируется только на Windows; на Linux/macOS автопроверка смотрит `.desktop`, systemd unit files и LaunchAgents.

## Запуск

1. `cubecheck.exe` — основное приложение. При первом запуске обычной Windows-сборки может запросить права администратора, чтобы создать `C:\Program Files\CubeCheck`. Portable/universal этого не делают.
2. `CubeCheck-Setup.exe` — установщик: кладёт программу в `C:\Program Files\CubeCheck`, копирует `assets\tools.json` и иконку, создаёт ярлыки.

Утилиты обычной сборки качаются в `C:\Program Files\CubeCheck\assets` из раздела «Компоненты». В **universal-local** они уже лежат в `payload/windows-*/assets/`.

## Конфигурация и данные

| Путь | Назначение |
|------|------------|
| `C:\Program Files\CubeCheck\cubecheck.exe` | Установленная обычная Windows-сборка |
| `C:\Program Files\CubeCheck\settings.json` | Настройки (тема, свечение, масштаб, автосохранение) |
| `C:\Program Files\CubeCheck\assets\` | Скачанные утилиты и `tools.json` |
| `C:\Program Files\CubeCheck\reports\` | Сохранённые отчёты |
| рядом с exe (portable / universal) | `settings.json`, `assets/`, `reports/` |

Шаблон настроек: `assets/settings.default.json`. Список загрудок: `assets/tools.json`. Список файлов для офлайн-пака: `scripts/vendor-files.txt`.

## Структура проекта

```
src/                 исходники приложения
src/bin/setup.rs     установщик Windows
crates/cubecheck-launcher/  лаунчер universal (только std)
src/scan/            автопроверка и список имён читов
src/tools/           запуск и пути утилит
src/ui/              интерфейс (egui)
assets/              иконка, tools.json, настройки по умолчанию
scripts/             build.ps1, pack-deb.sh, pack-linux-sh.sh, debian/, posix-launcher.sh
build.rs             иконка Windows, копирование ресурсов
build.bat / build.sh сборка: dist/ (staging), build/ (GitHub Releases)
dist/                промежуточные деревья и рецепты
build/               плоские файлы для вкладки Releases
```

## Сторонние программы

CubeCheck **не включает** лицензии сторонних утилит в MIT. В обычных **Windows**-сборках они скачиваются с официальных адресов при работе программы и распространяются своими авторами.

На **Linux/macOS** сторонние GUI (FSearch, Mission Center, sysdig, Catfish) **не скачиваются приложением**. Их ставят из репозитория дистрибутива или Flatpak (подсказки в «Компонентах»). Встроенные панели CubeCheck (автозагрузка, недавние файлы) — часть MIT-кода CubeCheck.

В **universal-local** Windows-файлы **копируются в комплект** с вашей локальной машины (`assets/`, в git они не входят). Их лицензии остаются лицензиями издателей. **Не утверждается, что MIT CubeCheck покрывает Everything, Sysinternals, System Informer или Shellbag.** Чтобы легально раздавать universal-local, вам может понадобиться самим принять условия издателей и собрать этот пак у себя.

| Программа | ОС | Издатель | Сайт / лицензия |
|-----------|----|----------|-----------------|
| [Everything](https://www.voidtools.com/) | Windows | voidtools PTY LTD | собственная лицензия voidtools |
| [Process Monitor](https://learn.microsoft.com/sysinternals/downloads/procmon), [Autoruns](https://learn.microsoft.com/sysinternals/downloads/autoruns), [Process Explorer](https://learn.microsoft.com/sysinternals/downloads/process-explorer) | Windows | Microsoft | [Sysinternals Software License Terms](https://learn.microsoft.com/sysinternals/license-terms) |
| [System Informer](https://github.com/winsiderss/systeminformer) | Windows | Winsider Seminars & Solutions | лицензия проекта System Informer (см. репозиторий) |
| [Shellbag Analyzer](https://privazer.com/) | Windows | Goversoft LLC | лицензия PrivaZer / Goversoft |
| [FSearch](https://github.com/cboxdoerfer/fsearch) | Linux | Christian Boxdörfer | GPL-2.0+ |
| [Catfish](https://docs.xfce.org/apps/catfish/start) | Linux | Xfce | GPL-2.0+ |
| plocate / mlocate | Linux | пакет дистрибутива | GPL (см. пакет) |
| [Mission Center](https://missioncenter.io/) | Linux | Mission Center Authors | GPL-3.0 |
| GNOME System Monitor, Tweaks | Linux | GNOME | GPL |
| [sysdig](https://github.com/draios/sysdig) | Linux | Sysdig | лицензия проекта sysdig (Apache-2.0 / GPL для kmod — см. репозиторий) |
| Spotlight (`mdfind`), Activity Monitor, `fs_usage`, Trash | macOS | Apple | часть macOS, не распространяются CubeCheck |

Не копируйте Windows-`.exe` в git: для обычной сборки они качаются во время работы. Локальные копии в `assets/` игнорируются `.gitignore`. Сборка `universal-local` читает их оттуда.

Что пакуется в Windows-payload **universal-local** (см. `scripts/vendor-files.txt`): Everything.exe, Shellbag.exe, Procmon64.exe, Autoruns64.exe, procexp64.exe, SystemInformer.exe + ksi.dll / .sys / подписи. `Everything.db` (локальный индекс) **не** включается.

## Лицензия

Исходный код CubeCheck — [MIT](LICENSE.md), © 2026 AuraStudio, AnProject.

Сторонние утилиты, перечисленные выше, **не** покрываются лицензией MIT.
