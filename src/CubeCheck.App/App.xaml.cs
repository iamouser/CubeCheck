using System.Windows;
using CubeCheck;

namespace CubeCheck.App;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
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
        base.OnStartup(e);
    }
}
