using System.Threading.Tasks;
using System.Windows;
using CubeCheck;

namespace CubeCheck.App;

public partial class App : Application
{
    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        NativeLoader.Ensure();
        try
        {
            AppPaths.EnsureInstallDir();
        }
        catch (Exception ex)
        {
            try { Native.MessageBox("CubeCheck", ex.Message); }
            catch { MessageBox.Show(ex.Message, "CubeCheck"); }
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
                        Shutdown();
                        return;
                    }
                    catch
                    {
                        // network failed: open the app, keep the offer as an icon
                    }
                }
            }
        }

        var window = new MainWindow();
        MainWindow = window;
        window.Show();
        if (early != null) window.ShowUpdateOffer(early);
        if (pending != null)
        {
            var late = pending;
            _ = late.ContinueWith(t =>
            {
                UpdateOffer? offer = null;
                if (t.Status == TaskStatus.RanToCompletion) offer = t.Result;
                if (offer == null) return;
                window.Dispatcher.BeginInvoke(() => window.ShowUpdateOffer(offer));
            }, TaskScheduler.Default);
        }
        window.StartUpdateSchedule();
    }
}
