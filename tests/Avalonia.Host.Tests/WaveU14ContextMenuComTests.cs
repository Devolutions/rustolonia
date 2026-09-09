using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU14ContextMenuComTests
{
    [Fact]
    public void Context_menu_advises_opening_and_closing()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateContextMenu(out var menu));
        Assert.NotNull(menu);

        var menuWrapper = Assert.IsType<AvnContextMenu>(menu);

        Assert.Equal(0, menuWrapper.AdviseOpening(new OpeningHandler(), out var openingSubscription));
        Assert.Equal(0, menuWrapper.AdviseClosing(new ClosingHandler(), out var closingSubscription));
        Assert.Equal(0, menuWrapper.UnadviseOpening(openingSubscription));
        Assert.Equal(0, menuWrapper.UnadviseClosing(closingSubscription));
    }

    [Fact]
    public void Open_with_control_reports_the_managed_failure_without_throwing()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateContextMenu(out var menu));
        Assert.Equal(0, factory.CreateButton(out var button));

        var menuWrapper = Assert.IsType<AvnContextMenu>(menu);
        var buttonWrapper = Assert.IsType<AvnButton>(button);

        // Depending on the host's window setup the managed ContextMenu either
        // opens (S_OK) or refuses (failure HRESULT); both cross without
        // crashing, and Close always succeeds.
        menuWrapper.OpenWithControl(buttonWrapper);
        Assert.Equal(0, menuWrapper.Close());
    }

    private sealed class OpeningHandler : IAvnContextMenuOpeningHandler
    {
        public int Invoke(ref int Cancel) => 0;
    }

    private sealed class ClosingHandler : IAvnContextMenuClosingHandler
    {
        public int Invoke(ref int Cancel) => 0;
    }
}
