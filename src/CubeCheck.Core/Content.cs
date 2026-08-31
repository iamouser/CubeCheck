namespace CubeCheck;

public static class Content
{
    public const string AppVersion = "1.1 beta";
    public const string Authors = "AuraStudio, AnProject";
    public const string TelegramUrl = "https://telegram.me/cubecheck";
    public const string HolyCheckUrl = "https://mods.holyworld.me/";

    public const string AboutText =
        """
        CubeCheck — проверка компьютера на читы Minecraft.

        Что проверяет:
        • запущенные программы
        • файлы на рабочем столе, в загрузках и в .minecraft
        • автозагрузку
        • логи Minecraft
        • сведения об ОС и корзину

        Программы внутри (зависят от ОС):
        Windows: Everything, Shellbag Analyzer, System Informer,
        Process Monitor, Autoruns, Process Explorer.
        Linux: FSearch/Catfish/plocate, недавние файлы, Mission Center,
        sysdig, автозагрузка systemd/.desktop, монитор системы.
        macOS: Spotlight (mdfind), недавние файлы, Activity Monitor,
        fs_usage, Login Items/LaunchAgents, Корзина.

        Авторы: AuraStudio, AnProject
        Канал: @cubecheck
        Версия: 1.1 beta
        """;
}
