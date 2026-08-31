using System.Collections.Concurrent;
using System.Globalization;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Styling;
using Avalonia.Threading;
using CubeCheck;

namespace CubeCheck.Desktop;

enum ViewKind
{
    Util,
    Recycle,
    Components,
    AutoCheck,
    About,
    System,
    Settings
}

enum ComponentStatusKind
{
    Ready,
    Missing,
    Downloading,
    Verifying,
    Extracting,
    Failed
}

sealed class ComponentStatus
{
    public ComponentStatusKind Kind;
    public long Received;
    public long? Total;
    public string? Error;
}

public partial class MainWindow : Window
{
    AppConfig _config;
    ThemeColors _colors = ThemeColors.For(ThemeId.Black);
    ViewKind _view = ViewKind.Util;
    int _utilIndex;
    bool _programsOpen;
    readonly List<string> _findings = [];
    bool _scanStarted;
    string? _scanPhase;
    readonly ConcurrentDictionary<string, ComponentStatus> _components = new();
    int _downloadBusy;
    bool _autoDownloadStarted;
    bool _exitSaved;
    AppConfig? _resetSnapshot;
    DateTime _undoExpires;
    DispatcherTimer? _glowTimer;
    DispatcherTimer? _undoTick;
    double _glowPhase;
    Action? _dialogOk;
    Action? _dialogCancel;
    bool _dialogAllowDimClose;
    string? _dialogLinkUrl;
    bool _scanBusy;
    bool _resetSaveOnExit;
    double _pickerH, _pickerS, _pickerV;
    Action<byte[]>? _pickerSet;
    Border? _pickerSwatch;
    bool _svDrag, _hueDrag;

    public MainWindow()
    {
        InitializeComponent();
        AddHandler(KeyDownEvent, OnPreviewKeyDown, RoutingStrategies.Tunnel);
        _config = AppConfig.Load();
        foreach (var util in Catalog.Utils)
        {
            _components[util.Id] = new ComponentStatus
            {
                Kind = AppPaths.ToolInstalled(util.Id) ? ComponentStatusKind.Ready : ComponentStatusKind.Missing
            };
        }

        _view = AppPaths.AnyToolMissing() ? ViewKind.Components : ViewKind.Util;
        ApplyTheme(_config.ThemeId, persist: false);
        ApplyZoom(_config.Zoom);
        BuildProgramsList();
        ShowView(_view);
        FooterVersion.Text = $"CubeCheck {CubeCheck.Content.AppVersion}";
        FooterAuthors.Text = $"авторы: {CubeCheck.Content.Authors}";
        AboutBody.Text = CubeCheck.Content.AboutText;
        LoadWindowIcon();
        Closing += (_, _) => SaveOnExit();
        Closed += (_, _) => SaveOnExit();
        StartGlowTimer();
        Dispatcher.UIThread.Post(MaybeAutodownload, DispatcherPriority.Background);
    }

    public void ShowStartupAlert(string msg) => Alert(msg, true);

    void LoadWindowIcon()
    {
        var ico = Path.Combine(AppContext.BaseDirectory, "assets", "cubecheck.ico");
        if (!File.Exists(ico)) return;
        try
        {
            using var stream = File.OpenRead(ico);
            Icon = new WindowIcon(stream);
        }
        catch
        {
            // keep default
        }
    }

    void StartGlowTimer()
    {
        _glowTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(40) };
        _glowTimer.Tick += (_, _) =>
        {
            if (_config.Glow is { Enabled: true, Gradient: true })
            {
                _glowPhase += 0.04 * Math.Max(0.1, _config.Glow.GradientSpeed);
                ApplyGlowEffects();
            }
        };
        _glowTimer.Start();
    }

    void ApplyTheme(ThemeId id, bool persist)
    {
        _colors = ThemeColors.For(id);
        _config.SetTheme(id);
        PaintChrome();
        if (persist) PersistAfterChange();
        RefreshCurrentView();
    }

    void PaintChrome()
    {
        Background = ThemeHelper.Brush(_colors.Bg);
        Sidebar.Background = ThemeHelper.Brush(_colors.Bg);
        ContentPane.Background = ThemeHelper.Brush(_colors.Card);
        ContentPane.BorderBrush = ThemeHelper.Brush(_colors.Border);
        ContentPane.BorderThickness = new Thickness(_colors.Light ? 1.5 : 1);
        Footer.Background = ThemeHelper.Brush(_colors.Bg);
        Foreground = ThemeHelper.Brush(_colors.Fg);

        void PaintText(TextBlock? tb, Rgb rgb)
        {
            if (tb != null) tb.Foreground = ThemeHelper.Brush(rgb);
        }

        PaintText(SidebarTitle, _colors.Fg);
        PaintText(SecPrograms, _colors.Section);
        if (ProgramsArrow != null)
            ProgramsArrow.Fill = ThemeHelper.Brush(_colors.Section);
        PaintText(SecCheck, _colors.Section);
        PaintText(SecComponents, _colors.Section);
        PaintText(SecSettings, _colors.Section);
        PaintText(SecInfo, _colors.Section);
        PaintText(AutoCheckTitle, _colors.Fg);
        PaintText(SystemTitle, _colors.Fg);
        PaintText(AboutBody, _colors.TextDim);
        PaintText(FooterVersion, _colors.Footer);
        PaintText(FooterTelegram, _colors.Footer);
        PaintText(FooterAuthors, _colors.Footer);
        PaintText(FooterSep1, _colors.TextDim);
        PaintText(FooterSep2, _colors.TextDim);
        foreach (var sep in new[] { Sep1, Sep2, Sep3, Sep4 })
        {
            sep.Background = ThemeHelper.Brush(_colors.Border);
        }

        AutoCheckBody.Background = ThemeHelper.Brush(_colors.Card);
        AutoCheckBody.Foreground = ThemeHelper.Brush(_colors.TextDim);
        PaintAccent(BtnSysInfo);
        PaintAccent(BtnReset);
        PaintAccent(BtnUndo);
        PaintAccent(DialogOk);
        PaintAccent(DialogCancel);
        DialogCard.Background = ThemeHelper.Brush(_colors.Card);
        DialogCard.BorderBrush = ThemeHelper.Brush(_colors.Border);
        DialogTitle.Foreground = ThemeHelper.Brush(_colors.Fg);
        DialogBody.Foreground = ThemeHelper.Brush(_colors.Fg);
        DialogLink.Foreground = ThemeHelper.Brush(_colors.Accent);
        UndoToast.Background = ThemeHelper.Brush(_colors.ButtonBg);
        UndoToast.BorderBrush = ThemeHelper.Brush(_colors.Border);
        UndoLabel.Foreground = ThemeHelper.Brush(_colors.Fg);
        UndoSecs.Foreground = ThemeHelper.Brush(_colors.TextDim);

        StyleNav(BtnProgramsToggle, false);
        StyleNav(BtnAutoCheck, _view == ViewKind.AutoCheck);
        StyleNav(BtnSaveReport, false);
        StyleNav(BtnClearLogs, false);
        StyleNav(BtnComponents, _view == ViewKind.Components);
        StyleNav(BtnSettings, _view == ViewKind.Settings);
        StyleNav(BtnAbout, _view == ViewKind.About);
        StyleNav(BtnSystem, _view == ViewKind.System);
        StyleNav(BtnHoly, false);
        SetAppBrush("CcTrack", _colors.Track);
        SetAppBrush("CcBorder", _colors.Border);
        SetAppBrush("CcAccent", _colors.Accent);
        SetAppBrush("CcHandle", _colors.Handle);
        SetAppBrush("CcOutline", _colors.WidgetOutline);
        SetAppBrush("CcInput", _colors.InputBg);
        SetAppBrush("CcSelect", _colors.Select);
        SetAppBrush("CcFg", _colors.Fg);
        ApplyGlowEffects();
    }

    static void SetAppBrush(string key, Rgb rgb)
    {
        if (Application.Current != null)
        {
            Application.Current.Resources[key] = new SolidColorBrush(ThemeHelper.ToMedia(rgb));
        }
    }

    ControlTheme ThemeOf(string key) => (ControlTheme)this.FindResource(key)!;

    void StyleNav(Button btn, bool selected)
    {
        btn.Tag = selected;
        btn.Background = selected ? ThemeHelper.Brush(_colors.Select) : Brushes.Transparent;
        btn.Foreground = ThemeHelper.Brush(selected ? _colors.Fg : _colors.TextDim);
        btn.BorderBrush = selected ? ThemeHelper.Brush(_colors.Accent) : Brushes.Transparent;
        btn.BorderThickness = selected ? new Thickness(1.5) : new Thickness(0);
        btn.PointerEntered -= NavHoverIn;
        btn.PointerExited -= NavHoverOut;
        btn.PointerEntered += NavHoverIn;
        btn.PointerExited += NavHoverOut;
    }

    void NavHoverIn(object? sender, PointerEventArgs e)
    {
        if (sender is Button btn && btn.Tag is not true)
        {
            btn.Background = ThemeHelper.Brush(_colors.Hover);
        }
    }

    void NavHoverOut(object? sender, PointerEventArgs e)
    {
        if (sender is Button btn && btn.Tag is not true)
        {
            btn.Background = Brushes.Transparent;
        }
    }

    void PaintAccent(Button btn)
    {
        btn.Background = ThemeHelper.Brush(_colors.ButtonBg);
        btn.Foreground = ThemeHelper.Brush(_colors.Fg);
        btn.BorderBrush = ThemeHelper.Brush(_colors.WidgetOutline);
        btn.PointerEntered -= AccentHoverIn;
        btn.PointerExited -= AccentHoverOut;
        btn.PointerEntered += AccentHoverIn;
        btn.PointerExited += AccentHoverOut;
    }

    void AccentHoverIn(object? sender, PointerEventArgs e)
    {
        if (sender is Button btn) btn.Background = ThemeHelper.Brush(_colors.Hover);
    }

    void AccentHoverOut(object? sender, PointerEventArgs e)
    {
        if (sender is Button btn) btn.Background = ThemeHelper.Brush(_colors.ButtonBg);
    }

    void ApplyGlowEffects()
    {
        ApplyGlow(Sidebar, GlowArea.Sidebar);
        ApplyGlow(Footer, GlowArea.Footer);
        ApplyGlow(ViewAbout, GlowArea.About);
        ApplyGlow(ViewSystem, GlowArea.System);
    }

    Color CurrentGlowColor()
    {
        var c1 = ThemeHelper.ToMedia(_config.Glow.Color, new Rgb(212, 175, 55));
        var c2 = ThemeHelper.ToMedia(_config.Glow.Color2, new Rgb(255, 214, 90));
        if (!_config.Glow.Gradient) return c1;
        var t = (Math.Sin(_glowPhase) + 1) * 0.5;
        return Color.FromRgb(
            (byte)(c1.R + (c2.R - c1.R) * t),
            (byte)(c1.G + (c2.G - c1.G) * t),
            (byte)(c1.B + (c2.B - c1.B) * t));
    }

    void ApplyGlow(Border target, GlowArea area)
    {
        if (_config.Glow.ActiveFor(area))
        {
            var c = CurrentGlowColor();
            var opacity = Polyfill.Clamp(_config.Glow.Intensity * 0.55, 0.1, 1.0);
            target.BoxShadow = new BoxShadows(new BoxShadow
            {
                Blur = Math.Max(8, _config.Glow.Radius),
                Color = Color.FromArgb((byte)(opacity * 255), c.R, c.G, c.B)
            });
        }
        else
        {
            target.BoxShadow = default;
        }
    }

    void FollowGlow(Border glowLayer, Point pos, Size size, GlowArea area)
    {
        if (!_config.Glow.ActiveFor(area) || size.Width < 1 || size.Height < 1)
        {
            glowLayer.Background = null;
            return;
        }
        var color = CurrentGlowColor();
        var radius = Math.Max(20, _config.Glow.Radius * 1.8);
        glowLayer.Background = new RadialGradientBrush
        {
            GradientOrigin = new RelativePoint(pos.X / size.Width, pos.Y / size.Height, RelativeUnit.Relative),
            Center = new RelativePoint(pos.X / size.Width, pos.Y / size.Height, RelativeUnit.Relative),
            RadiusX = new RelativeScalar(radius / size.Width, RelativeUnit.Relative),
            RadiusY = new RelativeScalar(radius / size.Height, RelativeUnit.Relative),
            GradientStops =
            {
                new GradientStop(color, 0),
                new GradientStop(Colors.Transparent, 1)
            },
            Opacity = Polyfill.Clamp(_config.Glow.Intensity * 0.35, 0.05, 0.8)
        };
    }

    void OnSidebarPointerMoved(object? sender, PointerEventArgs e) =>
        FollowGlow(SidebarGlow, e.GetPosition(Sidebar), Sidebar.Bounds.Size, GlowArea.Sidebar);

    void OnFooterPointerMoved(object? sender, PointerEventArgs e) =>
        FollowGlow(FooterGlow, e.GetPosition(Footer), Footer.Bounds.Size, GlowArea.Footer);

    void OnAboutPointerMoved(object? sender, PointerEventArgs e) =>
        FollowGlow(AboutGlow, e.GetPosition(ViewAbout), ViewAbout.Bounds.Size, GlowArea.About);

    void OnSystemPointerMoved(object? sender, PointerEventArgs e) =>
        FollowGlow(SystemGlow, e.GetPosition(ViewSystem), ViewSystem.Bounds.Size, GlowArea.System);

    void ApplyZoom(float zoom)
    {
        zoom = AppConfig.ClampZoom(zoom);
        _config.SetZoom(zoom);
        ZoomHost.LayoutTransform = new ScaleTransform(zoom, zoom);
    }

    void PersistAfterChange()
    {
        if (_config.Autosave == AutosaveMode.OnChange)
        {
            TrySaveSettings();
        }
    }

    void TrySaveSettings()
    {
        try { _config.Save(); }
        catch (Exception ex) { Alert(ex.Message, true); }
    }

    void SaveOnExit()
    {
        if (_exitSaved) return;
        _exitSaved = true;
        if (_config.Autosave == AutosaveMode.OnExit || _resetSaveOnExit)
        {
            try { _config.Save(); } catch { /* ignore */ }
        }
    }

    void Alert(string msg, bool error)
    {
        var wide = msg.Contains('\\') || msg.Contains('/') || msg.Length > 42;
        DialogCard.Width = wide ? 460 : 320;
        DialogTitle.Text = error ? "Ошибка" : "Успешно";
        DialogBody.Text = SoftWrap(msg);
        DialogLink.IsVisible = false;
        DialogOk.Content = "OK";
        DialogCancel.IsVisible = false;
        _dialogAllowDimClose = true;
        _dialogLinkUrl = null;
        _dialogOk = HideDialog;
        _dialogCancel = HideDialog;
        DialogOverlay.IsVisible = true;
    }

    static string SoftWrap(string msg)
    {
        return msg.Replace("\\", "\\\u200B").Replace("/", "/\u200B");
    }

    void Confirm(string title, string body, string okText, string cancelText, Action ok, string? link = null)
    {
        DialogCard.Width = 320;
        DialogTitle.Text = title;
        DialogBody.Text = body;
        DialogOk.Content = okText;
        DialogCancel.Content = cancelText;
        DialogCancel.IsVisible = true;
        if (!string.IsNullOrEmpty(link))
        {
            DialogLink.Text = link;
            DialogLink.IsVisible = true;
            _dialogLinkUrl = link;
        }
        else
        {
            DialogLink.IsVisible = false;
            _dialogLinkUrl = null;
        }
        _dialogAllowDimClose = true;
        _dialogOk = () => { HideDialog(); ok(); };
        _dialogCancel = HideDialog;
        DialogOverlay.IsVisible = true;
    }

    void HideDialog()
    {
        DialogOverlay.IsVisible = false;
        _dialogOk = null;
        _dialogCancel = null;
    }

    void OnDialogOk(object? sender, RoutedEventArgs e) => _dialogOk?.Invoke();
    void OnDialogCancel(object? sender, RoutedEventArgs e) => _dialogCancel?.Invoke();

    void OnDialogDimClick(object? sender, PointerPressedEventArgs e)
    {
        if (_dialogAllowDimClose) _dialogCancel?.Invoke();
    }

    void OnDialogLink(object? sender, PointerReleasedEventArgs e)
    {
        if (!string.IsNullOrEmpty(_dialogLinkUrl))
        {
            ToolLauncher.OpenUrl(_dialogLinkUrl);
        }
    }

    void OnPreviewKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.KeyModifiers != KeyModifiers.Control) return;
        if (e.Key is Key.OemPlus or Key.Add)
        {
            ApplyZoom(_config.Zoom + 0.1f);
            PersistAfterChange();
            e.Handled = true;
        }
        else if (e.Key is Key.OemMinus or Key.Subtract)
        {
            ApplyZoom(_config.Zoom - 0.1f);
            PersistAfterChange();
            e.Handled = true;
        }
        else if (e.Key is Key.D0 or Key.NumPad0)
        {
            ApplyZoom(1f);
            PersistAfterChange();
            e.Handled = true;
        }
    }

    void ShowView(ViewKind view)
    {
        _view = view;
        ViewUtils.IsVisible = view is ViewKind.Util or ViewKind.Recycle;
        ViewComponents.IsVisible = view == ViewKind.Components;
        ViewAutoCheck.IsVisible = view == ViewKind.AutoCheck;
        ViewAbout.IsVisible = view == ViewKind.About;
        ViewSystem.IsVisible = view == ViewKind.System;
        ViewSettings.IsVisible = view == ViewKind.Settings;
        PaintChrome();
        RefreshCurrentView();
    }

    void RefreshCurrentView()
    {
        switch (_view)
        {
            case ViewKind.Util:
            case ViewKind.Recycle:
                RebuildUtils();
                break;
            case ViewKind.Components:
                RebuildComponents();
                break;
            case ViewKind.AutoCheck:
                RebuildAutoCheck();
                break;
            case ViewKind.System:
                RebuildSystem();
                break;
            case ViewKind.Settings:
                RebuildSettings();
                break;
        }
        BuildProgramsList();
    }

    void BuildProgramsList()
    {
        if (ProgramsArrowRot != null)
            ProgramsArrowRot.Angle = _programsOpen ? 90 : 0;
        ProgramsList.Children.Clear();
        ProgramsList.IsVisible = _programsOpen;
        for (var i = 0; i < Catalog.Utils.Length; i++)
        {
            var idx = i;
            var util = Catalog.Utils[i];
            var btn = new Button { Content = util.Name, Theme = ThemeOf("NavButton") };
            StyleNav(btn, _view == ViewKind.Util && _utilIndex == i);
            btn.Click += (_, _) =>
            {
                _utilIndex = idx;
                ShowView(ViewKind.Util);
            };
            ProgramsList.Children.Add(btn);
        }
        var recycle = new Button { Content = "Корзина", Theme = ThemeOf("NavButton") };
        StyleNav(recycle, _view == ViewKind.Recycle);
        recycle.Click += (_, _) => ShowView(ViewKind.Recycle);
        ProgramsList.Children.Add(recycle);
    }

    Button Accent(string text)
    {
        var btn = new Button
        {
            Content = text,
            Theme = ThemeOf("AccentButton"),
            Width = 110,
            Height = 32,
            Margin = new Thickness(6, 0, 0, 0)
        };
        PaintAccent(btn);
        return btn;
    }

    Control ProgressTrack(double frac, string text)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
        var track = new Border
        {
            Width = 160,
            Height = 9,
            CornerRadius = new CornerRadius(3),
            Background = ThemeHelper.Brush(_colors.Track),
            BorderBrush = ThemeHelper.Brush(_colors.Border),
            BorderThickness = new Thickness(1),
            Margin = new Thickness(12, 0, 0, 0)
        };
        var fillW = Math.Max(frac > 0 ? 4 : 0, (160 - 2) * Polyfill.Clamp(frac, 0, 1));
        if (fillW > 0)
        {
            track.Child = new Border
            {
                Width = fillW,
                Height = 7,
                CornerRadius = new CornerRadius(2),
                Background = ThemeHelper.Brush(_colors.Accent),
                HorizontalAlignment = HorizontalAlignment.Left
            };
        }
        row.Children.Add(track);
        if (!string.IsNullOrEmpty(text))
        {
            row.Children.Add(new TextBlock
            {
                Text = text,
                FontSize = 11,
                Foreground = ThemeHelper.Brush(_colors.TextDim),
                Margin = new Thickness(8, 0, 0, 0),
                VerticalAlignment = VerticalAlignment.Center
            });
        }
        return row;
    }

    Border Card(bool selected)
    {
        return new Border
        {
            CornerRadius = new CornerRadius(6),
            Padding = new Thickness(14),
            Margin = new Thickness(0, 0, 0, 8),
            BorderThickness = new Thickness(1),
            Background = ThemeHelper.Brush(selected ? _colors.Select : _colors.ButtonBg),
            BorderBrush = ThemeHelper.Brush(selected ? _colors.Accent : _colors.Border)
        };
    }

    void RebuildUtils()
    {
        UtilsHost.Children.Clear();
        UtilsHost.Children.Add(new TextBlock
        {
            Text = "УТИЛИТЫ",
            FontSize = 22,
            FontWeight = FontWeight.Bold,
            Foreground = ThemeHelper.Brush(_colors.Fg)
        });
        UtilsHost.Children.Add(new TextBlock
        {
            Text = "Нажмите «Открыть».",
            FontSize = 13,
            Foreground = ThemeHelper.Brush(_colors.TextDim),
            Margin = new Thickness(0, 4, 0, 12)
        });

        for (var i = 0; i < Catalog.Utils.Length; i++)
        {
            var util = Catalog.Utils[i];
            var selected = _view == ViewKind.Util && _utilIndex == i;
            var installed = AppPaths.ToolInstalled(util.Id);
            var card = Card(selected);
            var grid = new Grid();
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            var left = new StackPanel();
            var titleRow = new StackPanel { Orientation = Orientation.Horizontal };
            titleRow.Children.Add(new TextBlock
            {
                Text = util.Name,
                FontSize = 16,
                FontWeight = FontWeight.Bold,
                Foreground = ThemeHelper.Brush(_colors.Fg)
            });
            titleRow.Children.Add(new TextBlock
            {
                Text = installed ? "  установлен" : "  не установлен",
                FontSize = 12,
                Foreground = ThemeHelper.Brush(installed ? _colors.Accent : _colors.TextDim),
                VerticalAlignment = VerticalAlignment.Center
            });
            left.Children.Add(titleRow);
            left.Children.Add(new TextBlock
            {
                Text = util.Desc,
                FontSize = 12,
                Foreground = ThemeHelper.Brush(_colors.TextDim),
                Margin = new Thickness(0, 4, 0, 0),
                TextWrapping = TextWrapping.Wrap
            });
            Grid.SetColumn(left, 0);
            grid.Children.Add(left);

            var actions = new StackPanel { Orientation = Orientation.Horizontal };
            if (util.IsSearch)
            {
                var copy = Accent("Копировать список");
                copy.Width = 150;
                ToolTip.SetTip(copy, "Скопировать названия читов");
                copy.Click += (_, _) =>
                {
                    CopyText(Catalog.CheatListText());
                    Alert("Список скопирован", false);
                };
                actions.Children.Add(copy);
            }
            var open = Accent(installed ? "ОТКРЫТЬ" : "Скачать");
            var id = util.Id;
            var name = util.Name;
            open.Click += (_, _) =>
            {
                if (installed)
                {
                    try { ToolLauncher.RunUtil(id); }
                    catch (Exception ex) { Alert(ex.Message, true); }
                }
                else if (Downloader.DownloadsEnabled)
                {
                    ShowView(ViewKind.Components);
                    StartDownloads([id], false);
                }
                else if (AppPaths.IsOffline)
                {
                    Alert($"{name} нет в assets/. Офлайн-сборка не качает файлы из сети.", true);
                }
                else
                {
                    Alert("Эта утилита работает только в Windows.", true);
                }
            };
            actions.Children.Add(open);
            Grid.SetColumn(actions, 1);
            grid.Children.Add(actions);
            card.Child = grid;
            var idx = i;
            card.PointerReleased += (_, e) =>
            {
                if (e.InitialPressMouseButton == MouseButton.Left)
                {
                    _utilIndex = idx;
                    ShowView(ViewKind.Util);
                }
            };
            UtilsHost.Children.Add(card);
        }

        var recycleSelected = _view == ViewKind.Recycle;
        var recycle = Card(recycleSelected);
        var rg = new Grid();
        rg.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        rg.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        var rleft = new StackPanel();
        rleft.Children.Add(new TextBlock
        {
            Text = "Корзина",
            FontSize = 16,
            FontWeight = FontWeight.Bold,
            Foreground = ThemeHelper.Brush(_colors.Fg)
        });
        rleft.Children.Add(new TextBlock
        {
            Text = "Открыть корзину Windows.",
            FontSize = 12,
            Foreground = ThemeHelper.Brush(_colors.TextDim),
            Margin = new Thickness(0, 4, 0, 0)
        });
        Grid.SetColumn(rleft, 0);
        rg.Children.Add(rleft);
        var ropen = Accent("Открыть корзину");
        ropen.Click += (_, _) =>
        {
            try { ToolLauncher.OpenRecycleBin(); }
            catch (Exception ex) { Alert(ex.Message, true); }
        };
        Grid.SetColumn(ropen, 1);
        rg.Children.Add(ropen);
        recycle.Child = rg;
        recycle.PointerReleased += (_, e) =>
        {
            if (e.InitialPressMouseButton == MouseButton.Left) ShowView(ViewKind.Recycle);
        };
        UtilsHost.Children.Add(recycle);
    }

    void CopyText(string text)
    {
        var clipboard = TopLevel.GetTopLevel(this)?.Clipboard;
        if (clipboard != null)
        {
            _ = clipboard.SetTextAsync(text);
        }
    }

    void RebuildComponents()
    {
        ComponentsHost.Children.Clear();
        ComponentsHost.Children.Add(new TextBlock
        {
            Text = "КОМПОНЕНТЫ",
            FontSize = 22,
            FontWeight = FontWeight.Bold,
            Foreground = ThemeHelper.Brush(_colors.Fg)
        });
        var hint = AppPaths.IsOffline
            ? "Офлайн-сборка: файлы берутся из папки assets, без загрузки из сети."
            : "Скачивание программ с официальных сайтов.";
        ComponentsHost.Children.Add(new TextBlock
        {
            Text = hint,
            FontSize = 13,
            Foreground = ThemeHelper.Brush(_colors.TextDim),
            Margin = new Thickness(0, 4, 0, 10)
        });

        var missing = PendingDownloadIds();
        var row = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 0, 0, 10) };
        var all = Accent("Скачать все");
        all.Click += (_, _) => StartDownloads(missing, false);
        row.Children.Add(all);
        if (System.Threading.Volatile.Read(ref _downloadBusy) != 0)
        {
            row.Children.Add(new TextBlock
            {
                Text = "  Скачивается...",
                FontSize = 13,
                Foreground = ThemeHelper.Brush(_colors.Accent),
                VerticalAlignment = VerticalAlignment.Center
            });
        }
        ComponentsHost.Children.Add(row);

        foreach (var util in Catalog.Utils)
        {
            var status = _components.GetOrAdd(util.Id, _ => new ComponentStatus { Kind = ComponentStatusKind.Missing });
            if (AppPaths.ToolInstalled(util.Id) && status.Kind is ComponentStatusKind.Missing or ComponentStatusKind.Failed)
            {
                status.Kind = ComponentStatusKind.Ready;
            }
            ComponentsHost.Children.Add(BuildInstallCard(util, status));
        }
    }

    Border BuildInstallCard(Util util, ComponentStatus status)
    {
        var card = Card(false);
        var grid = new Grid();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        var name = new TextBlock
        {
            Text = util.Name,
            FontSize = 16,
            FontWeight = FontWeight.Bold,
            Foreground = ThemeHelper.Brush(_colors.Fg),
            VerticalAlignment = VerticalAlignment.Center
        };
        Grid.SetColumn(name, 0);
        grid.Children.Add(name);

        var (stext, scolor) = status.Kind switch
        {
            ComponentStatusKind.Ready => ("установлен", _colors.Accent),
            ComponentStatusKind.Failed => ("ошибка", new Rgb(220, 90, 90)),
            ComponentStatusKind.Downloading => ("загрузка", _colors.Fg),
            ComponentStatusKind.Verifying => ("проверка", _colors.Fg),
            ComponentStatusKind.Extracting => ("распаковка", _colors.Fg),
            _ => ("не установлен", _colors.TextDim)
        };
        var mid = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(14, 0, 12, 0) };
        var sl = new TextBlock
        {
            Text = stext,
            FontSize = 12,
            Foreground = ThemeHelper.Brush(scolor),
            VerticalAlignment = VerticalAlignment.Center
        };
        if (status.Kind == ComponentStatusKind.Failed && !string.IsNullOrEmpty(status.Error))
        {
            ToolTip.SetTip(sl, status.Error);
        }
        mid.Children.Add(sl);
        if (status.Kind is ComponentStatusKind.Downloading or ComponentStatusKind.Verifying or ComponentStatusKind.Extracting)
        {
            var frac = status.Kind == ComponentStatusKind.Downloading
                ? status.Total is > 0 ? status.Received / (double)status.Total.Value : 0
                : 1;
            var mb = "";
            if (status.Received > 0)
            {
                mb = status.Total is > 0
                    ? $"{status.Received / 1048576.0:0.0} / {status.Total.Value / 1048576.0:0.0} МБ"
                    : $"{status.Received / 1048576.0:0.0} МБ";
            }
            mid.Children.Add(ProgressTrack(frac, mb));
        }
        Grid.SetColumn(mid, 1);
        grid.Children.Add(mid);

        Button? action = status.Kind switch
        {
            ComponentStatusKind.Ready => Accent("ОТКРЫТЬ"),
            ComponentStatusKind.Failed => Accent("Повтор"),
            ComponentStatusKind.Missing => Accent("Скачать"),
            _ => null
        };
        if (action != null)
        {
            var id = util.Id;
            action.Click += (_, _) =>
            {
                if (status.Kind == ComponentStatusKind.Ready)
                {
                    try { ToolLauncher.RunUtil(id); }
                    catch (Exception ex) { Alert(ex.Message, true); }
                }
                else
                {
                    StartDownloads([id], status.Kind == ComponentStatusKind.Failed);
                }
            };
            Grid.SetColumn(action, 2);
            grid.Children.Add(action);
        }
        card.Child = grid;
        return card;
    }

    List<string> PendingDownloadIds() =>
        Catalog.Utils
            .Where(u => !AppPaths.ToolInstalled(u.Id))
            .Where(u =>
            {
                if (!_components.TryGetValue(u.Id, out var st)) return true;
                return st.Kind is not (ComponentStatusKind.Ready or ComponentStatusKind.Downloading);
            })
            .Select(u => u.Id)
            .ToList();

    void StartDownloads(List<string> ids, bool force)
    {
        if (!Downloader.DownloadsEnabled)
        {
            Alert(AppPaths.IsOffline
                ? "Офлайн-сборка: загрузка из сети отключена. Файлы должны лежать в assets/."
                : "Сторонние .exe скачиваются только в Windows.", true);
            return;
        }
        ids = ids.Where(id => force || !AppPaths.ToolInstalled(id)).ToList();
        if (ids.Count == 0)
        {
            Alert("Всё уже скачано", false);
            return;
        }
        if (Interlocked.CompareExchange(ref _downloadBusy, 1, 0) != 0)
        {
            Alert("Уже скачивается", false);
            return;
        }
        foreach (var id in ids)
        {
            _components[id] = new ComponentStatus { Kind = ComponentStatusKind.Downloading };
        }
        RebuildComponents();
        Task.Run(() =>
        {
            try
            {
                var manifest = Downloader.LoadManifest();
                foreach (var id in ids)
                {
                    var spec = manifest.Get(id);
                    if (spec == null)
                    {
                        _components[id] = new ComponentStatus { Kind = ComponentStatusKind.Failed, Error = "Нет в списке загрузок" };
                        continue;
                    }
                    try
                    {
                        Downloader.DownloadTool(spec, force, p =>
                        {
                            var kind = p.Kind switch
                            {
                                ToolProgressKind.Verifying => ComponentStatusKind.Verifying,
                                ToolProgressKind.Extracting => ComponentStatusKind.Extracting,
                                _ => ComponentStatusKind.Downloading
                            };
                            _components[id] = new ComponentStatus
                            {
                                Kind = kind,
                                Received = p.Received,
                                Total = p.Total
                            };
                            Dispatcher.UIThread.Post(RebuildComponents);
                        });
                        _components[id] = new ComponentStatus { Kind = ComponentStatusKind.Ready };
                    }
                    catch (Exception ex)
                    {
                        _components[id] = new ComponentStatus { Kind = ComponentStatusKind.Failed, Error = ex.Message };
                    }
                }
            }
            catch (Exception ex)
            {
                foreach (var id in ids)
                {
                    _components[id] = new ComponentStatus { Kind = ComponentStatusKind.Failed, Error = ex.Message };
                }
            }
            finally
            {
                Interlocked.Exchange(ref _downloadBusy, 0);
                Dispatcher.UIThread.Post(RebuildComponents);
            }
        });
    }

    void MaybeAutodownload()
    {
        if (_autoDownloadStarted || _view != ViewKind.Components) return;
        _autoDownloadStarted = true;
        if (!Downloader.DownloadsEnabled) return;
        try
        {
            var missing = Downloader.MissingTools(Downloader.LoadManifest()).Select(t => t.Id).ToList();
            if (missing.Count > 0) StartDownloads(missing, false);
        }
        catch
        {
            // keep UI usable
        }
    }

    void RebuildAutoCheck()
    {
        var text = "";
        if (_scanPhase != null) text += $"Сканирование: {_scanPhase}...\n";
        else if (!_scanStarted) text += "Нажмите «Автопроверка» слева.";
        if (_scanStarted && _scanPhase == null)
        {
            var findings = _findings.Where(l => !l.StartsWith("КОРЗИНА", StringComparison.OrdinalIgnoreCase)).ToList();
            if (findings.Count == 0)
            {
                text += "Ничего подозрительного не найдено\n";
                foreach (var line in _findings) text += line + "\n";
            }
            else
            {
                text += "Найдено:\n\n";
                for (var i = 0; i < _findings.Count; i++)
                {
                    text += $"№{i + 1}  {_findings[i]}\n";
                }
            }
            text += $"\n{Catalog.AutocheckSearchStatusLine}\n";
        }
        AutoCheckBody.Text = text;
    }

    void RebuildSystem()
    {
        SetLabeled(SysUser, "Пользователь: ", SystemInfo.UserName);
        SetLabeled(SysComputer, "Имя компьютера: ", SystemInfo.ComputerName);
        SetLabeled(SysWindows, SystemInfo.OsInfoLabel + ": ", SystemInfo.WindowsInstallDate);
    }

    void SetLabeled(TextBlock tb, string label, string value)
    {
        tb.Inlines ??= new InlineCollection();
        tb.Inlines.Clear();
        tb.Inlines.Add(DimLabel(label));
        tb.Inlines.Add(Strong(value));
    }

    Run DimLabel(string t) =>
        new(t) { Foreground = ThemeHelper.Brush(_colors.Section), FontSize = 13 };

    Run Strong(string t) =>
        new(t) { Foreground = ThemeHelper.Brush(_colors.Fg), FontSize = 14, FontWeight = FontWeight.Bold };

    void RebuildSettings()
    {
        ColorPopup.IsOpen = false;
        SettingsHost.Children.Clear();
        SettingsHost.Children.Add(new TextBlock
        {
            Text = "НАСТРОЙКИ",
            FontSize = 22,
            FontWeight = FontWeight.Bold,
            Foreground = ThemeHelper.Brush(_colors.Fg),
            Margin = new Thickness(0, 0, 0, 16)
        });

        SettingsHost.Children.Add(Section("Тема"));
        var themes = new UniformGrid { Columns = ThemeColors.All.Length, Margin = new Thickness(0, 0, 0, 16) };
        foreach (var theme in ThemeColors.All)
        {
            var t = theme;
            var chip = Accent(ThemeColors.Label(t));
            chip.Width = double.NaN;
            chip.Margin = new Thickness(0, 0, 8, 0);
            chip.HorizontalAlignment = HorizontalAlignment.Stretch;
            if (t == _config.ThemeId)
            {
                chip.Background = ThemeHelper.Brush(_colors.Select);
                chip.BorderBrush = ThemeHelper.Brush(_colors.Accent);
            }
            chip.Click += (_, _) =>
            {
                ApplyTheme(t, persist: true);
                RebuildSettings();
            };
            themes.Children.Add(chip);
        }
        SettingsHost.Children.Add(themes);

        SettingsHost.Children.Add(Section("Подсветка"));
        SettingsHost.Children.Add(CheckRow("Включена", _config.Glow.Enabled, v =>
        {
            _config.Glow.Enabled = v;
            AfterGlowChange();
        }));
        SettingsHost.Children.Add(ColorRow("Цвет", _config.Glow.Color, rgb =>
        {
            _config.Glow.Color = rgb;
            AfterGlowChange();
        }));
        if (_config.Glow.Gradient)
        {
            SettingsHost.Children.Add(ColorRow("Цвет 2", _config.Glow.Color2, rgb =>
            {
                _config.Glow.Color2 = rgb;
                AfterGlowChange();
            }));
        }
        SettingsHost.Children.Add(CheckRow("Градиент", _config.Glow.Gradient, v =>
        {
            _config.Glow.Gradient = v;
            AfterGlowChange();
            RebuildSettings();
        }));
        if (_config.Glow.Gradient)
        {
            SettingsHost.Children.Add(SliderRow("Скорость", _config.Glow.GradientSpeed, AppConfig.GlowSpeedMin, AppConfig.GlowSpeedMax, v =>
            {
                _config.Glow.GradientSpeed = v;
                AfterGlowChange();
            }));
        }
        SettingsHost.Children.Add(SliderRow("Радиус", _config.Glow.Radius, AppConfig.GlowRadiusMin, AppConfig.GlowRadiusMax, v =>
        {
            _config.Glow.Radius = v;
            AfterGlowChange();
        }));
        SettingsHost.Children.Add(SliderRow("Интенсивность", _config.Glow.Intensity, AppConfig.GlowIntensityMin, AppConfig.GlowIntensityMax, v =>
        {
            _config.Glow.Intensity = v;
            AfterGlowChange();
        }));

        SettingsHost.Children.Add(Section("Области"));
        var areas = new WrapPanel { Margin = new Thickness(0, 0, 0, 16) };
        areas.Children.Add(AreaChip("Меню", _config.Glow.Areas.Sidebar, v => _config.Glow.Areas.Sidebar = v));
        areas.Children.Add(AreaChip("О программе", _config.Glow.Areas.About, v => _config.Glow.Areas.About = v));
        areas.Children.Add(AreaChip("Система", _config.Glow.Areas.System, v => _config.Glow.Areas.System = v));
        areas.Children.Add(AreaChip("Подвал", _config.Glow.Areas.Footer, v => _config.Glow.Areas.Footer = v));
        SettingsHost.Children.Add(areas);

        SettingsHost.Children.Add(Section("Автосохранение"));
        SettingsHost.Children.Add(RadioRow("при выключении программы", AutosaveMode.OnExit));
        SettingsHost.Children.Add(RadioRow("при изменении настроек", AutosaveMode.OnChange));
        SettingsHost.Children.Add(RadioRow("не сохранять", AutosaveMode.Off));

        SettingsHost.Children.Add(Section("Масштаб"));
        SettingsHost.Children.Add(SliderRow("Масштаб", _config.Zoom, AppConfig.ZoomMin, AppConfig.ZoomMax, v =>
        {
            ApplyZoom(v);
            PersistAfterChange();
        }));
    }

    TextBlock Section(string title) => new()
    {
        Text = title,
        FontSize = 16,
        FontWeight = FontWeight.Bold,
        Foreground = ThemeHelper.Brush(_colors.Fg),
        Margin = new Thickness(0, 8, 0, 10)
    };

    Control CheckRow(string label, bool value, Action<bool> set)
    {
        var row = new Grid { Margin = new Thickness(0, 0, 0, 6), Height = 30 };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(152) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.Children.Add(new TextBlock
        {
            Text = label,
            FontSize = 13.5,
            Foreground = ThemeHelper.Brush(_colors.Fg),
            VerticalAlignment = VerticalAlignment.Center
        });
        var box = new CheckBox
        {
            IsChecked = value,
            VerticalAlignment = VerticalAlignment.Center,
            Theme = ThemeOf("CcCheck")
        };
        Grid.SetColumn(box, 1);
        box.IsCheckedChanged += (_, _) => set(box.IsChecked == true);
        row.Children.Add(box);
        return row;
    }

    Control AreaChip(string label, bool value, Action<bool> set)
    {
        var box = new CheckBox
        {
            Content = label,
            IsChecked = value,
            Theme = ThemeOf("CcCheck"),
            Foreground = ThemeHelper.Brush(_colors.Fg),
            Margin = new Thickness(0, 0, 16, 8)
        };
        box.IsCheckedChanged += (_, _) => { set(box.IsChecked == true); AfterGlowChange(); };
        return box;
    }

    Control ColorRow(string label, byte[] rgb, Action<byte[]> set)
    {
        var row = new Grid { Margin = new Thickness(0, 0, 0, 8), Height = 30 };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(152) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        row.Children.Add(new TextBlock
        {
            Text = label,
            FontSize = 13.5,
            Foreground = ThemeHelper.Brush(_colors.Fg),
            VerticalAlignment = VerticalAlignment.Center
        });
        var preview = new Border
        {
            Width = 26,
            Height = 22,
            Cursor = new Cursor(StandardCursorType.Hand),
            Background = ThemeHelper.Brush(Color.FromRgb(rgb[0], rgb[1], rgb[2])),
            BorderBrush = ThemeHelper.Brush(_colors.WidgetOutline),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(4),
            VerticalAlignment = VerticalAlignment.Center
        };
        preview.PointerReleased += (_, e) =>
        {
            if (e.InitialPressMouseButton == MouseButton.Left)
            {
                OpenColorPicker(preview, rgb, set);
            }
        };
        Grid.SetColumn(preview, 1);
        row.Children.Add(preview);
        return row;
    }

    Control SliderRow(string label, float value, float min, float max, Action<float> set)
    {
        var row = new Grid { Margin = new Thickness(0, 0, 0, 8) };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(152) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(58) });
        row.Children.Add(new TextBlock
        {
            Text = label,
            Foreground = ThemeHelper.Brush(_colors.Fg),
            VerticalAlignment = VerticalAlignment.Center
        });
        var slider = new Slider
        {
            Minimum = min,
            Maximum = max,
            Value = value,
            VerticalAlignment = VerticalAlignment.Center,
            Theme = ThemeOf("CcSlider")
        };
        Grid.SetColumn(slider, 1);
        var box = new TextBox
        {
            Text = value.ToString("0.##"),
            Width = 58,
            Background = ThemeHelper.Brush(_colors.InputBg),
            Foreground = ThemeHelper.Brush(_colors.Fg),
            BorderBrush = ThemeHelper.Brush(_colors.WidgetOutline)
        };
        Grid.SetColumn(box, 2);
        slider.ValueChanged += (_, e) =>
        {
            var v = (float)e.NewValue;
            box.Text = v.ToString("0.##");
            set(v);
        };
        void ApplyBox()
        {
            if (float.TryParse(box.Text.Replace(',', '.'), NumberStyles.Float,
                    CultureInfo.InvariantCulture, out var typed))
            {
                typed = Polyfill.Clamp(typed, min, max);
                slider.Value = typed;
                set(typed);
            }
            else
            {
                box.Text = slider.Value.ToString("0.##");
            }
        }
        box.LostFocus += (_, _) => ApplyBox();
        box.KeyDown += (_, e) =>
        {
            if (e.Key == Key.Enter) ApplyBox();
        };
        row.Children.Add(slider);
        row.Children.Add(box);
        return row;
    }

    Control RadioRow(string label, AutosaveMode mode)
    {
        var radio = new RadioButton
        {
            Content = label,
            GroupName = "autosave",
            IsChecked = _config.Autosave == mode,
            Theme = ThemeOf("CcRadio"),
            Foreground = ThemeHelper.Brush(_colors.Fg),
            Margin = new Thickness(0, 0, 0, 6)
        };
        radio.IsCheckedChanged += (_, _) =>
        {
            if (radio.IsChecked != true) return;
            var prev = _config.Autosave;
            _config.Autosave = mode;
            if (_config.Autosave == AutosaveMode.OnChange || prev == AutosaveMode.OnChange)
            {
                TrySaveSettings();
            }
        };
        return radio;
    }

    void AfterGlowChange()
    {
        _config.Glow.Sanitize();
        ApplyGlowEffects();
        PersistAfterChange();
    }

    void OnTogglePrograms(object? sender, RoutedEventArgs e)
    {
        _programsOpen = !_programsOpen;
        BuildProgramsList();
    }

    void OnAutoCheck(object? sender, RoutedEventArgs e)
    {
        if (_scanBusy) return;
        _scanBusy = true;
        _scanStarted = true;
        _findings.Clear();
        _scanPhase = "процессы";
        ShowView(ViewKind.AutoCheck);
        try { ToolLauncher.RunAutocheckSearch(); }
        catch (Exception ex) { Alert(ex.Message, true); }
        Task.Run(() =>
        {
            try
            {
                var results = Native.PerformScan(phase =>
                {
                    var label = phase switch
                    {
                        0 => "процессы",
                        1 => "файлы",
                        2 => "реестр",
                        3 => "логи",
                        _ => "сканирование"
                    };
                    Dispatcher.UIThread.Post(() =>
                    {
                        _scanPhase = label;
                        RebuildAutoCheck();
                    });
                });
                Dispatcher.UIThread.Post(() =>
                {
                    _findings.Clear();
                    _findings.AddRange(results);
                    _scanPhase = null;
                    RebuildAutoCheck();
                });
            }
            catch (Exception ex)
            {
                Dispatcher.UIThread.Post(() => Alert(ex.Message, true));
            }
            finally
            {
                Dispatcher.UIThread.Post(() =>
                {
                    _scanBusy = false;
                    if (_scanPhase != null)
                    {
                        _scanPhase = null;
                        RebuildAutoCheck();
                    }
                });
            }
        });
    }

    void OnSaveReport(object? sender, RoutedEventArgs e)
    {
        try
        {
            var path = ReportWriter.Save(_findings);
            Alert($"Отчёт сохранён: {path}", false);
        }
        catch (Exception ex)
        {
            Alert($"Не удалось сохранить отчёт: {ex.Message}", true);
        }
    }

    void OnClearLogs(object? sender, RoutedEventArgs e)
    {
        Confirm("Логи", "Удалить логи Minecraft?", "Очистить", "Отмена", () =>
        {
            try
            {
                ToolLauncher.ClearMinecraftLogs();
                Alert("Логи удалены", false);
            }
            catch (Exception ex)
            {
                Alert(ex.Message, true);
            }
        });
    }

    void OnComponents(object? sender, RoutedEventArgs e)
    {
        ShowView(ViewKind.Components);
        MaybeAutodownload();
    }

    void OnSettings(object? sender, RoutedEventArgs e) => ShowView(ViewKind.Settings);
    void OnAbout(object? sender, RoutedEventArgs e) => ShowView(ViewKind.About);
    void OnSystem(object? sender, RoutedEventArgs e) => ShowView(ViewKind.System);

    void OnHolyCheck(object? sender, RoutedEventArgs e)
    {
        Confirm("HolyCheck", "Открыть сайт HolyWorld?", "Да", "Отмена",
            ToolLauncher.OpenHolyCheck, CubeCheck.Content.HolyCheckUrl);
    }

    void OnSystemInfo(object? sender, RoutedEventArgs e)
    {
        try { ToolLauncher.RunSystemInfo(); }
        catch (Exception ex) { Alert(ex.Message, true); }
    }

    void OnTelegram(object? sender, PointerReleasedEventArgs e)
    {
        if (e.InitialPressMouseButton == MouseButton.Left)
        {
            ToolLauncher.OpenTelegram();
        }
    }

    void OnTelegramEnter(object? sender, PointerEventArgs e)
    {
        FooterTelegram.Foreground = ThemeHelper.Brush(_colors.Accent);
        FooterTelegram.TextDecorations = TextDecorations.Underline;
    }

    void OnTelegramLeave(object? sender, PointerEventArgs e)
    {
        FooterTelegram.Foreground = ThemeHelper.Brush(_colors.Footer);
        FooterTelegram.TextDecorations = null;
    }

    void OnResetSettings(object? sender, RoutedEventArgs e)
    {
        UndoToast.IsVisible = false;
        Confirm("Сброс настроек", "Вернуть настройки по умолчанию?", "Сбросить", "Отмена", ConfirmReset);
    }

    void ConfirmReset()
    {
        _resetSnapshot = _config.Clone();
        var prev = _resetSnapshot.Autosave;
        _resetSaveOnExit = prev == AutosaveMode.OnExit;
        _config = new AppConfig();
        ApplyTheme(_config.ThemeId, persist: false);
        ApplyZoom(_config.Zoom);
        if (prev == AutosaveMode.OnChange) TrySaveSettings();
        ShowUndoToast();
        if (_view == ViewKind.Settings) RebuildSettings();
    }

    void ShowUndoToast()
    {
        _undoExpires = DateTime.Now.AddSeconds(10);
        UndoToast.IsVisible = true;
        _undoTick?.Stop();
        _undoTick = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(200) };
        _undoTick.Tick += (_, _) =>
        {
            var left = _undoExpires - DateTime.Now;
            if (left <= TimeSpan.Zero)
            {
                HideUndoToast();
                return;
            }
            UndoSecs.Text = $"{Math.Ceiling(left.TotalSeconds)} с";
        };
        _undoTick.Start();
        UndoSecs.Text = "10 с";
    }

    void HideUndoToast()
    {
        _undoTick?.Stop();
        UndoToast.IsVisible = false;
        _resetSnapshot = null;
    }

    void OnUndoReset(object? sender, RoutedEventArgs e)
    {
        if (_resetSnapshot == null) return;
        _config = _resetSnapshot;
        _resetSaveOnExit = false;
        HideUndoToast();
        ApplyTheme(_config.ThemeId, persist: false);
        ApplyZoom(_config.Zoom);
        if (_config.Autosave == AutosaveMode.OnChange) TrySaveSettings();
        if (_view == ViewKind.Settings) RebuildSettings();
    }

    void OpenColorPicker(Border swatch, byte[] rgb, Action<byte[]> set)
    {
        _pickerSwatch = swatch;
        _pickerSet = set;
        RgbToHsv(rgb[0], rgb[1], rgb[2], out var h, out var s, out var v);
        if (s < 1.0 / 255) h = _pickerH;
        _pickerH = h;
        _pickerS = s;
        _pickerV = v;
        UpdatePickerVisuals();
        ColorPopup.PlacementTarget = swatch;
        ColorPopup.IsOpen = true;
    }

    void OnSvDown(object? sender, PointerPressedEventArgs e)
    {
        if (!e.GetCurrentPoint(SvCanvas).Properties.IsLeftButtonPressed) return;
        _svDrag = true;
        e.Pointer.Capture(SvCanvas);
        ApplySv(e.GetPosition(SvCanvas));
    }

    void OnSvMove(object? sender, PointerEventArgs e)
    {
        if (_svDrag) ApplySv(e.GetPosition(SvCanvas));
    }

    void OnSvUp(object? sender, PointerReleasedEventArgs e)
    {
        _svDrag = false;
        e.Pointer.Capture(null);
    }

    void OnHueDown(object? sender, PointerPressedEventArgs e)
    {
        if (!e.GetCurrentPoint(HueCanvas).Properties.IsLeftButtonPressed) return;
        _hueDrag = true;
        e.Pointer.Capture(HueCanvas);
        ApplyHue(e.GetPosition(HueCanvas));
    }

    void OnHueMove(object? sender, PointerEventArgs e)
    {
        if (_hueDrag) ApplyHue(e.GetPosition(HueCanvas));
    }

    void OnHueUp(object? sender, PointerReleasedEventArgs e)
    {
        _hueDrag = false;
        e.Pointer.Capture(null);
    }

    void ApplySv(Point pos)
    {
        _pickerS = Polyfill.Clamp(pos.X / 188.0, 0, 1);
        _pickerV = 1 - Polyfill.Clamp(pos.Y / 188.0, 0, 1);
        CommitPicker();
    }

    void ApplyHue(Point pos)
    {
        _pickerH = Polyfill.Clamp(pos.Y / 188.0, 0, 1);
        CommitPicker();
    }

    void CommitPicker()
    {
        var c = HsvToColor(_pickerH, _pickerS, _pickerV);
        var rgb = new byte[] { c.R, c.G, c.B };
        if (_pickerSwatch != null) _pickerSwatch.Background = ThemeHelper.Brush(c);
        _pickerSet?.Invoke(rgb);
        UpdatePickerVisuals();
    }

    void UpdatePickerVisuals()
    {
        if (SvHueRect.Fill is LinearGradientBrush hueBrush && hueBrush.GradientStops.Count > 1)
        {
            hueBrush.GradientStops[1].Color = HsvToColor(_pickerH, 1, 1);
        }
        Canvas.SetLeft(SvKnob, _pickerS * 188 - 7);
        Canvas.SetTop(SvKnob, (1 - _pickerV) * 188 - 7);
        SvKnob.Fill = ThemeHelper.Brush(HsvToColor(_pickerH, _pickerS, _pickerV));
        Canvas.SetTop(HueThumb, _pickerH * 188 - 5);
    }

    static void RgbToHsv(byte r, byte g, byte b, out double h, out double s, out double v)
    {
        var rd = r / 255.0;
        var gd = g / 255.0;
        var bd = b / 255.0;
        var max = Math.Max(rd, Math.Max(gd, bd));
        var min = Math.Min(rd, Math.Min(gd, bd));
        v = max;
        var d = max - min;
        s = max <= 0 ? 0 : d / max;
        if (d <= 0) h = 0;
        else if (max == rd) h = (((gd - bd) / d) + (gd < bd ? 6 : 0)) / 6;
        else if (max == gd) h = ((bd - rd) / d + 2) / 6;
        else h = ((rd - gd) / d + 4) / 6;
    }

    static Color HsvToColor(double h, double s, double v)
    {
        h = Polyfill.Clamp(h, 0, 1) * 6;
        var i = (int)Math.Floor(h);
        var f = h - i;
        var p = v * (1 - s);
        var q = v * (1 - f * s);
        var t = v * (1 - (1 - f) * s);
        var (rr, gg, bb) = (i % 6) switch
        {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q)
        };
        return Color.FromRgb(ToByte(rr), ToByte(gg), ToByte(bb));
    }

    static byte ToByte(double x) => (byte)Polyfill.Clamp((int)Math.Round(x * 255), 0, 255);
}
