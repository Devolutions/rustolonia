using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU21IconsComTests
{
    [Fact]
    public void Window_icon_crosses_as_a_write_oriented_path()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindow(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnWindow>(projected);

        // Reading yields null: a WindowIcon carries no source path.
        Assert.Equal(0, wrapper.GetIcon(out var none));
        Assert.Null(none);

        // A bogus path converts to null rather than throwing, and the slot
        // still round-trips.
        Assert.Equal(0, wrapper.SetIcon("does-not-exist.ico"));
        Assert.Equal(0, wrapper.GetIcon(out var stillNone));
        Assert.Null(stillNone);
    }

    [Fact]
    public void Tray_icon_icon_crosses_as_a_path()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTrayIcon(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTrayIcon>(projected);
        Assert.Equal(0, wrapper.SetIcon("app.ico"));
        Assert.Equal(0, wrapper.GetIcon(out var value));
        Assert.Null(value);
    }
}
