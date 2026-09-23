using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Threading;
using CubeCheck;

namespace CubeCheck.Desktop;

public partial class App : Application
{
    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override async void OnFrameworkInitializationCompleted()
    {
        NativeLoader.Ensure();

        string? startupError = null;
        try
        {
            AppPaths.EnsureInstallDir();
        }
        catch (Exception ex)
        {
            startupError = ex.Message;
        }

        var config = AppConfig.Load();
        UpdateOffer? early = null;
        Task<UpdateOffer?>? pending = null;
        if (config.CheckUpdates && !AppPaths.IsOffline)
        {
            pending = AppUpdate.CheckAsync(CancellationToken.None);
            var finished = await Task.WhenAny(pending, Task.Delay(AppUpdate.StartupGrace)).ConfigureAwait(true);
            if (finished == pending)
            {
                early = await pending.ConfigureAwait(true);
                pending = null;
                if (early != null)
                {
                    try
                    {
                        var path = await AppUpdate.DownloadInstallerAsync(early.InstallerUrl, CancellationToken.None)
                            .ConfigureAwait(true);
                        AppUpdate.StartInstaller(path, AppPaths.DataDir);
                        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime exiting)
                        {
                            exiting.Shutdown();
                        }
                        return;
                    }
                    catch
                    {
                        // open normally
                    }
                }
            }
        }

        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            var window = new MainWindow();
            desktop.MainWindow = window;
            if (early != null) window.ShowUpdateOffer(early);
            if (!string.IsNullOrEmpty(startupError))
            {
                var msg = startupError;
                window.Opened += (_, _) => window.ShowStartupAlert(msg);
            }
            if (pending != null)
            {
                var late = pending;
                _ = late.ContinueWith(t =>
                {
                    UpdateOffer? offer = null;
                    if (t.Status == TaskStatus.RanToCompletion) offer = t.Result;
                    if (offer == null) return;
                    Dispatcher.UIThread.Post(() => window.ShowUpdateOffer(offer));
                }, TaskScheduler.Default);
            }
            window.StartUpdateSchedule();
        }

        base.OnFrameworkInitializationCompleted();
    }
}
