# CubeCheck

**CubeCheck** — программа, которая помогает проверить компьютер на читы Minecraft. Она ищет известные имена читов в процессах, файлах, автозагрузке и логах, сохраняет отчёт и запускает утилиты проверки.

Авторы: **[AuraStudio](https://telegram.me/cubecheck)**, **[AnProject](https://discord.gg/Dwqu8xmaEc)**. Версия: **1.1 beta**.

## Возможности

- Автопроверка: процессы, файлы (рабочий стол, загрузки, `.minecraft`), автозагрузка, логи Minecraft.
- Сохранение отчёта (YAML) в папку `reports`.
- Очистка логов Minecraft (`%APPDATA%\.minecraft\logs` / `~/.minecraft/logs` / macOS Application Support).
- Запуск утилит проверки (набор зависит от ОС — см. таблицу ниже).
- Раздел «Компоненты»: на Windows — загрузка по HTTPS с проверкой издателя (Authenticode); на Linux/macOS — подсказки пакетного менеджера, без скачивания чужих `.exe`.
- Сведения о системе, корзина, ссылка на HolyCheck (HolyWorld).
- Темы оформления, свечение интерфейса, масштаб, автосохранение настроек.
- Установщик Windows (WPF-мастер): копирует программу в выбранный каталог (по умолчанию `C:\Program Files\CubeCheck`), создаёт ярлыки. Онлайн-версия качает payload с GitHub; офлайн — zip уже внутри `.exe`. Утилиты в установщик не входят (кроме **offline-setup**).
- Удаление: `cubecheck.exe -uninstall` или `UnInstall.url`.

## Требования

- **Windows (GUI):** Windows 10/11, 64-bit. Для установщика — [.NET 8 Desktop Runtime](https://dotnet.microsoft.com/download/dotnet/8.0).
- **Linux:** GTK-поиск (FSearch или Catfish) из репозитория; автопроверка может использовать `plocate`. **macOS:** Spotlight, Activity Monitor, `fs_usage` — встроены.
- Лаунчер **universal** — Windows 7/10/11, Linux и macOS, если в комплекте есть payload для этой ОС.
- **Сборка из исходников:** [Rust](https://rustup.rs/) (UI), .NET SDK 8 (Core, Api, Installer), MSVC (native DLL). Linux ELF с Windows — [zig](https://ziglang.org/) + `rustup target add x86_64-unknown-linux-gnu`. macOS Mach-O с Windows **не** собирается (нет Apple SDK).

## Сборка

### Быстро

```bat
build.bat
```

или `build.bat all` — полный набор установщиков в `build\`.

Отладка UI:

```bat
cd ui
cargo run --release --bin cubecheck --features gui
```

### Артефакты (`build/` после `build.bat all`)

| Файл | Назначение |
|------|------------|
| `CubeCheck-1.1.0-beta-universal-windows-setup.exe` | онлайн-мастер, качает payload с GitHub |
| `CubeCheck-1.1.0-beta-universal-windows-offline-setup.exe` | тот же мастер, zip внутри, без HTTP |
| `CubeCheck-1.1.0-beta-universal-linux-setup.run` | установщик Linux (ELF внутри) |
| `CubeCheck-1.1.0-beta-universal-linux-offline-setup.run` | Linux + `assets/bin`, без загрузок |
| `CubeCheck-1.1.0-beta-universal-macos-setup.run` | скрипт установки macOS |
| `CubeCheck-1.1.0-beta-universal-macos-offline-setup.run` | Darwin portables в `assets/bin` |
| `CubeCheck-1.1.0-beta-universal-macos-README.txt` | если Mach-O не собран на Windows |
| `CubeCheck-1.1.0-beta-github-payload.zip` | payload для репозитория `CubeCheck-payload` |

Другие цели: `build.bat github`, `windows-x64`, `linux-x64`, `universal`, `universal-local`.

Staging — `dist/`. В git не попадают `build/`, `dist/`, бинарники в корне.

### GitHub Releases

Загружайте **файлы из `build/`**, не деревья из `dist/`.

**Payload** (отдельный репозиторий): `CubeCheck-1.1.0-beta-github-payload.zip` → `jumpworlds/CubeCheck-payload`. Онлайн-мастер качает:

`https://github.com/jumpworlds/CubeCheck-payload/archive/refs/heads/main.zip`

**Linux:** `CubeCheck-1.1.0-beta-universal-linux-setup.run` — запускаемый установщик, внутри ELF `cubecheck` (без `.dll`).

```bash
chmod +x CubeCheck-1.1.0-beta-universal-linux-setup.run
./CubeCheck-1.1.0-beta-universal-linux-setup.run
```

**macOS:** `.run` + README. Mach-O `cubecheck` с Windows-ПК **не** собирается — честный бинарник только на Mac (`./build.sh macos-universal`, если есть).

## universal и universal-local

Один `.exe` не может одновременно быть PE, ELF и Mach-O. **universal** — переносной комплект:

```
CubeCheck-universal/
  cubecheck-launcher.exe   лаунчер Windows
  cubecheck.sh             лаунчер Linux/macOS
  payload/
    windows-x64/
    windows-x86/
    linux-x64/
    linux-x86/
    osx-x64/
    osx-arm64/
```

- **universal** — утилиты на Windows можно скачать в «Компонентах».
- **universal-local** — офлайн. Vendor-утилиты Windows копируются из локальной `assets/` при сборке (`scripts/vendor-files.txt`). Если файла нет — сборка падает.

Офлайн: фича `offline`, `CUBECHECK_OFFLINE=1` или файл `.offline` рядом с exe / в `assets/`.

Портативный режим: `CUBECHECK_PORTABLE=1` или `.portable`. Universal-сборки всегда портативные.

## Утилиты проверки по ОС

| Задача чекера | Windows | Linux | macOS |
|---------------|---------|-------|-------|
| Поиск имён на диске | [Everything](https://www.voidtools.com/) | [FSearch](https://github.com/cboxdoerfer/fsearch) + **plocate**/`locate` | **`mdfind`** (Spotlight) |
| Недавние папки/файлы | [Shellbag Analyzer](https://privazer.com/) | `~/.local/share/recently-used.xbel` | `mdfind` (`kMDItemLastUsedDate`) |
| Процессы | [System Informer](https://github.com/winsiderss/systeminformer) | [Mission Center](https://missioncenter.io/) / GNOME System Monitor | **Activity Monitor** |
| Живая активность | [Process Monitor](https://learn.microsoft.com/sysinternals/downloads/procmon) | **sysdig** / `lsof -r` | **`fs_usage`** |
| Автозагрузка | [Autoruns](https://learn.microsoft.com/sysinternals/downloads/autoruns) | autostart + systemd | Login Items + LaunchAgents |
| Дерево процессов | [Process Explorer](https://learn.microsoft.com/sysinternals/downloads/process-explorer) | htop / GNOME System Monitor | Activity Monitor |
| Корзина | Корзина Windows | XDG Trash | `~/.Trash` |

## Запуск

1. **`cubecheck.exe`** — основное приложение (Rust/egui + C# Core через `cubecheck_api.dll`).
2. **`CubeCheck-1.1.0-beta-universal-windows-setup.exe`** — установщик (WPF-мастер).
3. **`UnInstall.url`** или **`cubecheck.exe -uninstall`** — удаление с подтверждением.

Утилиты обычной сборки качаются в `assets\` из «Компонентов». В **offline-setup** / **universal-local** vendor-файлы уже в комплекте.

## Конфигурация и данные

| Путь | Назначение |
|------|------------|
| `C:\Program Files\CubeCheck\cubecheck.exe` | приложение |
| `C:\Program Files\CubeCheck\UnInstall.url` | ярлык удаления |
| `C:\Program Files\CubeCheck\settings.json` | настройки |
| `C:\Program Files\CubeCheck\assets\` | `cubecheck_api.dll`, `cubecheck_native.dll`, утилиты, `tools.json` |
| `C:\Program Files\CubeCheck\reports\` | отчёты |
| рядом с exe (portable) | `settings.json`, `assets/`, `reports/` |

`cubecheck_api.dll` и `cubecheck_native.dll` — **только** в `assets/`, не рядом с exe.

Шаблон: `assets/settings.default.json`. Список загрузок: `assets/tools.json`. Офлайн-пак: `scripts/vendor-files.txt`.

## Структура проекта

```
ui/                      интерфейс (Rust / egui), cubecheck.exe
src/CubeCheck.Core/      движок (C#)
src/CubeCheck.Api/       NativeAOT API (cubecheck_api.dll)
src/CubeCheck.Installer/ WPF-мастер установки
src/native/              C++ (cubecheck_native.dll)
crates/cubecheck-launcher/  лаунчер universal
assets/                  иконки, tools.json, шаблон настроек
scripts/build.ps1        сборка: dist/ (staging), build/ (релиз)
```

## Сторонние программы

CubeCheck **не включает** лицензии сторонних утилит в MIT. На Windows они скачиваются с официальных адресов (раздел «Компоненты»). На Linux/macOS — из репозитория дистрибутива.

В **offline** / **universal-local** Windows-файлы копируются в комплект с вашей машины (`assets/`, в git не входят). MIT CubeCheck **не** покрывает Everything, Sysinternals, System Informer, Shellbag.

| Программа | ОС | Издатель |
|-----------|----|----------|
| [Everything](https://www.voidtools.com/) | Windows | voidtools |
| [Process Monitor](https://learn.microsoft.com/sysinternals/downloads/procmon), [Autoruns](https://learn.microsoft.com/sysinternals/downloads/autoruns), [Process Explorer](https://learn.microsoft.com/sysinternals/downloads/process-explorer) | Windows | Microsoft (Sysinternals) |
| [System Informer](https://github.com/winsiderss/systeminformer) | Windows | Winsider |
| [Shellbag Analyzer](https://privazer.com/) | Windows | Goversoft |
| [FSearch](https://github.com/cboxdoerfer/fsearch), plocate | Linux | см. репозиторий / пакет |
| Spotlight, Activity Monitor, `fs_usage` | macOS | Apple |

Не копируйте vendor `.exe` в git. Список для offline: `scripts/vendor-files.txt`.

## Лицензия

Исходный код CubeCheck — [MIT](LICENSE.md), © 2026 AuraStudio, AnProject.

Сторонние утилиты **не** покрываются лицензией MIT.
