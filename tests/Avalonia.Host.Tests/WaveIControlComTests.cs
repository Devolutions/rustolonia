using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Notifications;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveIControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Popup_child_and_open_state_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePopup(out var projected));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedChild));
        Assert.NotNull(projected);
        Assert.NotNull(projectedChild);

        Through<IAvnPopup>(projected, popup =>
        {
            Assert.Equal(0, projectedChild.SetText("popup"));
            Assert.Equal(0, popup.SetChild((IAvnControl)projectedChild));
            Assert.Equal(0, popup.SetPlacement((int)PlacementMode.Bottom));
            Assert.Equal(0, popup.SetHorizontalOffset(8));
            Assert.Equal(0, popup.SetIsLightDismissEnabled(1));
        });

        var value = Target<Popup>(projected);
        Assert.Equal("popup", Assert.IsType<TextBlock>(value.Child).Text);
        Assert.Equal(PlacementMode.Bottom, value.Placement);
        Assert.True(value.IsLightDismissEnabled);
    }

    [Fact]
    public void Tray_icon_and_notifications_carry_scalars()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTrayIcon(out var projectedTray));
        Assert.Equal(0, factory.CreateWindowNotificationManager(out var projectedManager));
        Assert.Equal(0, factory.CreateNotificationCard(out var projectedCard));
        Assert.Equal(0, factory.CreateRefreshContainer(out var projectedRefresh));
        Assert.NotNull(projectedTray);
        Assert.NotNull(projectedManager);
        Assert.NotNull(projectedCard);
        Assert.NotNull(projectedRefresh);

        Through<IAvnTrayIcon>(projectedTray, tray =>
        {
            Assert.Equal(0, tray.SetToolTipText("Tray"));
            Assert.Equal(0, tray.SetIsVisible(0));
        });
        Through<IAvnWindowNotificationManager>(projectedManager, manager =>
        {
            Assert.Equal(0, manager.SetMaxItems(4));
            Assert.Equal(0, manager.SetPosition((int)NotificationPosition.BottomRight));
        });
        Through<IAvnRefreshContainer>(projectedRefresh, refresh =>
            Assert.Equal(0, refresh.SetIsMouseEnabled(0)));

        Assert.Equal("Tray", Target<TrayIcon>(projectedTray).ToolTipText);
        Assert.False(Target<TrayIcon>(projectedTray).IsVisible);
        Assert.Equal(4, Target<WindowNotificationManager>(projectedManager).MaxItems);
        Assert.False(Target<RefreshContainer>(projectedRefresh).IsMouseEnabled);
    }

    private static void Through<T>(object wrapper, Action<T> body) where T : class
    {
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(wrapper, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);
        try
        {
            body((T)s_wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None));
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
