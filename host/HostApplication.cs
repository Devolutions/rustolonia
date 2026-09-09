using Avalonia;
using Avalonia.Styling;
using Avalonia.Themes.Fluent;
#if AVN_DEVELOPER_TOOLS
using AvaloniaUI.DiagnosticsProtocol;
#endif

namespace Avalonia.Host;

public sealed class HostApplication : Application
{
    public override void Initialize()
    {
        Styles.Add(new FluentTheme());
        RequestedThemeVariant = ThemeVariant.Default;
#if AVN_DEVELOPER_TOOLS
        this.AttachDeveloperTools(options =>
            options.DiagnosticLogger = DiagnosticLogger.CreateConsole());
#endif
    }
}
