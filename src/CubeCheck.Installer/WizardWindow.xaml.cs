using System.ComponentModel;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Microsoft.Win32;

namespace CubeCheck.Installer;

public partial class WizardWindow : Window
{
    int _step = 1;
    bool _busy;
    bool _allowClose;
    bool _launchAfter = true;
    string _dest = InstallerConfig.DefaultDestination();
    readonly CancellationTokenSource _cts = new();

    public WizardWindow()
    {
        InitializeComponent();
        TxtWizardKind.Text = InstallerConfig.WizardSubtitle;
        LicenseBody.Text = LicenseText.Load();
        ChkMenu.Content = InstallerConfig.MenuShortcutLabel();
        TxtDest.Text = _dest;
        ApplyResumeArgs(Program.Args);
        ShowStep(_step);
        if (HasFlag(Program.Args, "--install") && ChkAccept.IsChecked == true)
        {
            Loaded += async (_, _) =>
            {
                _step = 4;
                ShowStep(_step);
                await RunInstallAsync();
            };
        }
    }

    void ApplyResumeArgs(string[] args)
    {
        if (args.Length == 0) return;
        var dest = ArgValue(args, "--dest");
        if (!string.IsNullOrWhiteSpace(dest))
        {
            _dest = dest;
            TxtDest.Text = dest;
        }
        if (HasFlag(args, "--accepted")) ChkAccept.IsChecked = true;
        if (HasFlag(args, "--no-desktop")) ChkDesktop.IsChecked = false;
        if (HasFlag(args, "--no-menu")) ChkMenu.IsChecked = false;
        if (HasFlag(args, "--no-launch")) ChkLaunch.IsChecked = false;
        if (HasFlag(args, "--desktop")) ChkDesktop.IsChecked = true;
        if (HasFlag(args, "--menu")) ChkMenu.IsChecked = true;
        if (HasFlag(args, "--launch")) ChkLaunch.IsChecked = true;
    }

    void OnAcceptChanged(object sender, RoutedEventArgs e) => UpdateButtons();

    void OnBrowse(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFolderDialog { Title = "Папка установки CubeCheck" };
        if (dlg.ShowDialog(this) != true) return;
        var path = dlg.FolderName;
        if (string.IsNullOrEmpty(path)) return;

        if (string.Equals(Path.GetFileName(path), "CubeCheck", StringComparison.OrdinalIgnoreCase) ||
            File.Exists(Path.Combine(path, "cubecheck.exe")) ||
            File.Exists(Path.Combine(path, "cubecheck")))
        {
            TxtDest.Text = path;
        }
        else
        {
            TxtDest.Text = Path.Combine(path, "CubeCheck");
        }
    }

    void OnBack(object sender, RoutedEventArgs e)
    {
        if (_busy || _step <= 1 || _step >= 5) return;
        _step--;
        ShowStep(_step);
    }

    void OnCancel(object sender, RoutedEventArgs e) => ShowCancelConfirm();

    void OnCancelNo(object sender, RoutedEventArgs e) => HideCancelConfirm();

    void OnCancelDim(object sender, MouseButtonEventArgs e) => HideCancelConfirm();

    void OnCancelYes(object sender, RoutedEventArgs e) => ConfirmCancelAndClose();

    void ShowCancelConfirm() => ConfirmOverlay.Visibility = Visibility.Visible;

    void HideCancelConfirm() => ConfirmOverlay.Visibility = Visibility.Collapsed;

    void ConfirmCancelAndClose()
    {
        _allowClose = true;
        if (_busy)
        {
            try { _cts.Cancel(); } catch { /* ignore */ }
        }
        Close();
    }

    void OnWindowClosing(object sender, CancelEventArgs e)
    {
        if (_allowClose || _step >= 5) return;
        e.Cancel = true;
        ShowCancelConfirm();
    }

    void OnPreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Escape || ConfirmOverlay.Visibility != Visibility.Visible) return;
        HideCancelConfirm();
        e.Handled = true;
    }

    async void OnNext(object sender, RoutedEventArgs e)
    {
        if (_step == 5)
        {
            Finish();
            return;
        }

        if (_step == 1 && ChkAccept.IsChecked != true) return;
        if (_step == 3)
        {
            _dest = (TxtDest.Text ?? "").Trim();
            _launchAfter = ChkLaunch.IsChecked == true;
            if (string.IsNullOrWhiteSpace(_dest)) return;
            if (OperatingSystem.IsWindows() && InstallerConfig.NeedsElevation(_dest) && !Elevation.IsElevated)
            {
                try
                {
                    Elevation.RelaunchElevated(BuildResumeArgs());
                    _allowClose = true;
                    Close();
                    return;
                }
                catch (Exception ex)
                {
                    ProgressError.Text = ex.Message;
                    ProgressError.Visibility = Visibility.Visible;
                    _step = 4;
                    ShowStep(_step);
                    return;
                }
            }
        }

        if (_step < 4)
        {
            _step++;
            ShowStep(_step);
            if (_step == 4) await RunInstallAsync();
            return;
        }

        if (_step == 4 && !_busy)
        {
            _step = 5;
            ShowStep(_step);
        }
    }

    async Task RunInstallAsync()
    {
        _busy = true;
        ProgressError.Visibility = Visibility.Collapsed;
        UpdateButtons();
        var options = new InstallOptions
        {
            Destination = (TxtDest.Text ?? _dest).Trim(),
            DesktopShortcut = ChkDesktop.IsChecked == true,
            MenuShortcut = ChkMenu.IsChecked == true,
            LaunchAfter = ChkLaunch.IsChecked == true,
            LicenseAccepted = ChkAccept.IsChecked == true
        };
        _dest = options.Destination;
        _launchAfter = options.LaunchAfter;

        var progress = new Progress<InstallProgress>(p =>
        {
            Dispatcher.Invoke(() =>
            {
                ProgressBar.Value = Math.Clamp(p.Percent, 0, 100);
                ProgressPct.Text = $"{p.Percent:0}%";
                ProgressStatus.Text = p.Status;
                ProgressFile.Text = p.CurrentFile;
            });
        });

        try
        {
            await Task.Run(() => InstallerEngine.InstallAsync(options, progress, _cts.Token));
            _busy = false;
            _step = 5;
            ShowStep(_step);
        }
        catch (OperationCanceledException)
        {
            _busy = false;
            if (!IsLoaded) return;
            ProgressError.Text = "Установка отменена.";
            ProgressError.Visibility = Visibility.Visible;
            UpdateButtons();
        }
        catch (Exception ex)
        {
            _busy = false;
            if (!IsLoaded) return;
            ProgressError.Text = ex.Message;
            ProgressError.Visibility = Visibility.Visible;
            UpdateButtons();
        }
    }

    void Finish()
    {
        if (_launchAfter)
        {
            try { InstallerEngine.Launch(_dest); }
            catch { /* ignore */ }
        }
        Close();
    }

    void ShowStep(int step)
    {
        StepLicense.Visibility = step == 1 ? Visibility.Visible : Visibility.Collapsed;
        StepShortcuts.Visibility = step == 2 ? Visibility.Visible : Visibility.Collapsed;
        StepDest.Visibility = step == 3 ? Visibility.Visible : Visibility.Collapsed;
        StepProgress.Visibility = step == 4 ? Visibility.Visible : Visibility.Collapsed;
        StepDone.Visibility = step == 5 ? Visibility.Visible : Visibility.Collapsed;
        PaintNav(1, Nav1, Nav1Text, step);
        PaintNav(2, Nav2, Nav2Text, step);
        PaintNav(3, Nav3, Nav3Text, step);
        PaintNav(4, Nav4, Nav4Text, step);
        PaintNav(5, Nav5, Nav5Text, step);
        UpdateButtons();
    }

    static readonly Brush NavSelect = Brush("#252540");
    static readonly Brush NavAccent = Brush("#8A7AD8");
    static readonly Brush NavFg = Brush("#E0E0F0");
    static readonly Brush NavGold = Brush("#D4AF37");
    static readonly Brush NavDim = Brush("#A0AABF");

    static SolidColorBrush Brush(string hex)
    {
        var b = new SolidColorBrush((Color)ColorConverter.ConvertFromString(hex)!);
        b.Freeze();
        return b;
    }

    void PaintNav(int index, Border border, TextBlock label, int current)
    {
        var active = index == current;
        var done = index < current;
        border.Background = active ? NavSelect : Brushes.Transparent;
        border.BorderBrush = active ? NavAccent : Brushes.Transparent;
        border.BorderThickness = new Thickness(active ? 1 : 0);
        label.Foreground = active ? NavFg : done ? NavGold : NavDim;
        label.FontWeight = active ? FontWeights.SemiBold : FontWeights.Normal;
    }

    void UpdateButtons()
    {
        BtnBack.IsEnabled = !_busy && _step is > 1 and < 5;
        BtnCancel.Visibility = _step < 5 ? Visibility.Visible : Visibility.Collapsed;
        BtnCancel.IsEnabled = !_busy || _step == 4;
        if (_step == 5)
        {
            BtnNext.Content = "Готово";
            BtnNext.IsEnabled = true;
            return;
        }
        if (_step == 3)
        {
            BtnNext.Content = "Установить";
            BtnNext.IsEnabled = !_busy && !string.IsNullOrWhiteSpace(TxtDest.Text);
            return;
        }
        if (_step == 4)
        {
            BtnNext.Content = "Далее";
            BtnNext.IsEnabled = false;
            return;
        }
        BtnNext.Content = "Далее";
        BtnNext.IsEnabled = _step != 1 || ChkAccept.IsChecked == true;
    }

    string[] BuildResumeArgs()
    {
        var list = new List<string>
        {
            "--install",
            "--accepted",
            "--dest",
            (TxtDest.Text ?? _dest).Trim()
        };
        list.Add(ChkDesktop.IsChecked == true ? "--desktop" : "--no-desktop");
        list.Add(ChkMenu.IsChecked == true ? "--menu" : "--no-menu");
        list.Add(ChkLaunch.IsChecked == true ? "--launch" : "--no-launch");
        return list.ToArray();
    }

    static bool HasFlag(string[] args, string flag) =>
        args.Any(a => string.Equals(a, flag, StringComparison.OrdinalIgnoreCase));

    static string? ArgValue(string[] args, string name)
    {
        for (var i = 0; i < args.Length - 1; i++)
        {
            if (string.Equals(args[i], name, StringComparison.OrdinalIgnoreCase))
            {
                return args[i + 1];
            }
        }
        return null;
    }
}
