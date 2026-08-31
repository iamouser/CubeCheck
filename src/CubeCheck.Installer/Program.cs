using System.Windows;

namespace CubeCheck.Installer;

static class Program
{
    public static string[] Args { get; private set; } = [];

    [STAThread]
    public static void Main(string[] args)
    {
        Args = args ?? [];
        var app = new App();
        app.InitializeComponent();
        app.Run(new WizardWindow());
    }
}
