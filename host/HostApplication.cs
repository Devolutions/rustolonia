using Avalonia;
using Avalonia.Styling;
using Avalonia.Themes.Fluent;

namespace Avalonia.Host;

public sealed class HostApplication : Application
{
    public override void Initialize()
    {
        Styles.Add(new FluentTheme());
        RequestedThemeVariant = ThemeVariant.Default;
    }
}
