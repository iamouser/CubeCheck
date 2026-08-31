CubeCheck Linux extras (offline)
================================

Авторы: AuraStudio, AnProject
Windows-утилиты (Everything, Sysinternals, System Informer) сюда не входят.

В payload/*/assets/bin уже лежат официальные переносимые бинарники.
Качать ничего не нужно. Скрипт install-linux-tools.sh не является продуктом —
это необязательная установка GTK-программ из репозитория дистрибутива.

Роли (уже в архиве):

  fd, rg, fzf     — поиск имён (аналог Everything)
  lf              — недавние файлы / файловый менеджер
  btop, btm, procs, Mission Center AppImage — процессы
  busybox / lsof  — открытые файлы (аналог Process Monitor)
  ~/.config/autostart — автозагрузка (открывается из CubeCheck)

FSearch (GTK) и sysdig (модуль ядра) нельзя честно уложить одним portable-файлом.
Вместо них в пакете: fd/fzf и lsof/busybox/btop.
