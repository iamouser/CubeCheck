using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using CubeCheck;

namespace CubeCheck.Desktop;

public partial class App : Application
{
    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
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

        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            var window = new MainWindow();
            desktop.MainWindow = window;
            if (!string.IsNullOrEmpty(startupError))
            {
                var msg = startupError;
                window.Opened += (_, _) => window.ShowStartupAlert(msg);
            }
        }

        base.OnFrameworkInitializationCompleted();
    }
}
