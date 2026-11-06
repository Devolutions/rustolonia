using Avalonia.Controls;
using Avalonia.Controls.Diagnostics;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class PopupHostingTests
{
    [Fact]
    public void Native_gate_rejects_unknown_tokens_and_has_a_stable_interface_shape()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = new AvnApplication();
        Assert.Equal(HResults.E_POINTER, application.SetNativePopupGate(null, null));
        Assert.Equal(HResults.E_INVALIDARG, application.AcknowledgeNativePopup(42));
        Assert.Equal(2, typeof(IAvnNativePopupHandler).GetMethods().Length);
    }

    [Fact]
    public void Overlay_policy_is_scoped_to_one_window_and_prevents_native_popup_hosts()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var protectedTarget = new Button();
        var normalTarget = new Button();
        var protectedWindow = new Window { Content = protectedTarget };
        var normalWindow = new Window { Content = normalTarget };
        var application = new AvnApplication();
        var projected = new AvnWindow(protectedWindow);

        Assert.Equal(HResults.E_POINTER, application.SetWindowOverlayPopups(null, 1));
        Assert.Equal(HResults.S_OK, application.SetWindowOverlayPopups(projected, 1));
        protectedWindow.Show();
        normalWindow.Show();
        var tip = new ToolTip { Content = "Protected" };
        ToolTip.SetTip(protectedTarget, tip);
        ToolTip.SetIsOpen(protectedTarget, true);
        Assert.IsType<OverlayPopupHost>(tip.PopupHost);
        ToolTip.SetIsOpen(protectedTarget, false);

        var flyout = new MenuFlyout();
        flyout.Items.Add(new MenuItem { Header = "Protected" });
        flyout.ShowAt(protectedTarget);
        Assert.IsType<OverlayPopupHost>(((IPopupHostProvider)flyout).PopupHost);
        Assert.True(protectedWindow.UseOverlayPopups);
        Assert.False(normalWindow.UseOverlayPopups);
        flyout.Hide();

        var normalTip = new ToolTip { Content = "Normal" };
        ToolTip.SetTip(normalTarget, normalTip);
        ToolTip.SetIsOpen(normalTarget, true);
        Assert.IsType<PopupRoot>(normalTip.PopupHost);
        ToolTip.SetIsOpen(normalTarget, false);
        Assert.Equal(HResults.S_OK, application.SetWindowOverlayPopups(projected, 0));
        ToolTip.SetIsOpen(protectedTarget, true);
        Assert.IsType<PopupRoot>(tip.PopupHost);
        ToolTip.SetIsOpen(protectedTarget, false);
    }
}
