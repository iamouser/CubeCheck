namespace CubeCheck;

public sealed record Util(string Id, string Name, string Desc)
{
    public bool IsSearch => Id is "everything" or "search";
}

public static class Catalog
{
    public static readonly Util[] WindowsUtils =
    [
        new("everything", "Everything", "Поиск файлов на компьютере."),
        new("shellbag", "Shellbag Analyzer", "Какие папки открывали в проводнике."),
        new("systeminformer", "System Informer", "Список запущенных программ."),
        new("procmon", "Process Monitor", "Что программы делают прямо сейчас."),
        new("autoruns", "Autoruns", "Что запускается вместе с Windows."),
        new("procexp", "Process Explorer", "Подробности о процессах.")
    ];

    public static readonly Util[] LinuxUtils =
    [
        new("search", "FSearch / Catfish", "Поиск файлов (FSearch, Catfish или plocate)."),
        new("files", "Недавние файлы", "Что недавно открывали в файловом менеджере."),
        new("processes", "Mission Center", "Список запущенных программ."),
        new("monitor", "sysdig / журнал", "Что программы делают прямо сейчас."),
        new("autoruns", "Автозагрузка", "systemd user и ~/.config/autostart."),
        new("procexp", "Монитор системы", "Подробности о процессах.")
    ];

    public static readonly Util[] MacUtils =
    [
        new("search", "Spotlight", "Поиск файлов через Spotlight (mdfind)."),
        new("files", "Недавние файлы", "Недавно открытые документы."),
        new("processes", "Activity Monitor", "Список запущенных программ."),
        new("monitor", "fs_usage", "Активность процессов (может запросить пароль)."),
        new("autoruns", "Login Items", "Точки входа и LaunchAgents."),
        new("procexp", "Activity Monitor", "Подробности о процессах.")
    ];

    public static Util[] Utils =>
        Compat.IsWindows ? WindowsUtils : Compat.IsMac ? MacUtils : LinuxUtils;

    public static readonly string[] CheatNames =
    [
        "impact", "wurst", "bleachhack", "aristois", "huzuni", "skillclient", "inertia", "ares", "sigma",
        "meteor", "liquidbounce", "nurik", "nursultan", "celestial", "calestial", "celka", "expensive",
        "neverhook", "excellent", "wexside", "wildclient", "minced", "deadcode", "akrien", "jigsaw",
        "jessica", "dreampool", "norules", "konas", "richclient", "rusherhack", "thunderhack",
        "moonhack", "doomsday", "nightware", "ricardo", "extazyy", "troxill", "antileak", "arbuz", ".akr",
        ".wex", "dauntiblyat", "rename_me_please", "editme", "takker", "fuzeclient", "wisefolder", "flauncher",
        "vec.dll", "USBOblivion.exe", "Feather", "venus", "baritone", "spambot", "CleanCut",
        "spam_bot", "inventory_walk", "player_highlighter", "aimbot", "freecam", "bedrock_breaker_mode",
        "viaversion", "double_hotbar", "elytra_swap", "armor_hotswap", "smart_moving", "savesearcher",
        "topkautobuy", "topkaautobuy", "tweakeroo", "mob_hitbox", "librarian_trade_finder", "sacurachorusfind",
        "autoattack", "entity_outliner", "invmove", "viabackwards", "viarewind", "viafabric", "viaforge",
        "viaproxy", "vialoader", "viamcp", "hitbox", "elytrahack", "DiamondSim", "ForgeHax", "clientcommands",
        "Control-Tweaks", "SwingThroughGrass", "CutThrough", "Haruka", "NewLauncher", "Blade", "Hachclient",
        "Fluger", "Exloader", "CatLean", "cproject", "eternity", "melonity", "relake", "rockstar", "verist",
        "zamorozka", "phobos", "pyro", "novoline", "vape", "astolfo", "koid", "nix", "spirt", "salhack",
        "gamesense"
    ];

    public static string EverythingSearchQuery(IEnumerable<string> terms) =>
        "(" + string.Join(" | ", terms) + ")";

    public static string CheatListText() => string.Join(" | ", CheatNames);

    public static int? UtilIndex(string id)
    {
        var utils = Utils;
        for (var i = 0; i < utils.Length; i++)
        {
            if (utils[i].Id == id) return i;
        }
        return null;
    }

    public static string AutocheckSearchStatusLine =>
        Compat.IsWindows
            ? "Everything открыт с поиском по читам."
            : Compat.IsMac
                ? "Spotlight / mdfind запущен с поиском по читам."
                : "Поиск Linux запущен с запросом по читам.";
}
