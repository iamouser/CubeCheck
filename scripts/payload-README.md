# CubeCheck payload

Бинарники CubeCheck **1.1 beta** для установщика. Исходников нет. AuraStudio, AnProject.

Онлайн-мастер Windows:

`https://github.com/jumpworlds/CubeCheck-payload/archive/refs/heads/main.zip`

Берёт из zip `windows-x64/`. Офлайн-`.exe` zip не качает.

## Содержимое

| Путь | |
|------|--|
| `LICENSE.md` | MIT |
| `SHA256SUMS` | |
| `install-linux.sh` | |
| `install-macos.sh` | |
| `windows-x64/` | |
| `linux-x64/` | |
| `linux-x86/` |

### windows-x64/

```
cubecheck.exe
UnInstall.url
.portable
assets/
  cubecheck_api.dll
  cubecheck_native.dll
  UnInstall.ico
  UnInstall.cmd
  cubecheck.ico
  tools.json
  settings.default.json
  Everything.ini
```

### linux-x64/, linux-x86/

```
cubecheck
cubecheck.sh
.portable
assets/
  cubecheck.ico
  tools.json
  settings.default.json
```

Сторонние `.exe` (Everything, Sysinternals и т.д.) **не** входят.
