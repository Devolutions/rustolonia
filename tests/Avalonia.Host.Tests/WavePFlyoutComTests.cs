using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WavePFlyoutComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Button_flyout_and_menu_item_open_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projectedButton));
        Assert.Equal(0, factory.CreateFlyout(out var projectedFlyout));
        Assert.Equal(0, factory.CreateMenuItem(out var projectedItem));
        Assert.NotNull(projectedButton);
        Assert.NotNull(projectedFlyout);
        Assert.NotNull(projectedItem);

        Through<IAvnButton>(projectedButton, button =>
            Assert.Equal(0, button.SetFlyout((IAvnFlyoutBase)projectedFlyout)));
        Through<IAvnMenuItem>(projectedItem, item =>
        {
            Assert.Equal(0, item.GetHasSubMenu(out _));
            Assert.Equal(0, item.GetIsTopLevel(out _));
            Assert.Equal(0, item.Close());
        });

        Assert.NotNull(Target<Button>(projectedButton).Flyout);
        Assert.NotNull(Target<MenuItem>(projectedItem));
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
