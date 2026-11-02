using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class ShellTrayTests
{
    [Fact]
    public void Code_first_tray_initializes_tooltip_before_attaching_and_normalizes_null()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = new AvnApplication();

        Assert.Equal(HResults.S_OK, application.CreateTrayIcon(null, out var tray));
        Assert.NotNull(tray);
        var icons = TrayIcon.GetIcons(Application.Current!);
        var icon = Assert.Single(icons!);
        Assert.Equal(string.Empty, icon.ToolTipText);
        Assert.Equal(HResults.S_OK, tray.SetToolTip("Tooltip"));
        Assert.Equal("Tooltip", icon.ToolTipText);
        Assert.Equal(HResults.S_OK, tray.SetToolTip(null));
        Assert.Equal(string.Empty, icon.ToolTipText);
        Assert.Equal(HResults.S_OK, tray.Remove());
        Assert.Empty(icons!);
    }
}
