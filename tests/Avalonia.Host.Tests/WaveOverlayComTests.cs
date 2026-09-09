using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveOverlayComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Popup_open_close_and_placement_target_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePopup(out var projectedPopup));
        Assert.Equal(0, factory.CreateButton(out var projectedButton));
        Assert.Equal(0, factory.CreateContextMenu(out var projectedMenu));
        Assert.NotNull(projectedPopup);
        Assert.NotNull(projectedButton);
        Assert.NotNull(projectedMenu);

        Through<IAvnPopup>(projectedPopup, popup =>
        {
            Assert.Equal(0, popup.SetInheritsTransform(1));
            Assert.Equal(0, popup.SetTakesFocusFromNativeControl(0));
            Assert.Equal(0, popup.SetShouldUseOverlayLayer(1));
            Assert.Equal(0, popup.GetIsPointerOverPopup(out _));
            Assert.Equal(0, popup.Close());
        });
        Through<IAvnContextMenu>(projectedMenu, menu =>
            Assert.Equal(0, menu.SetPlacementTarget((IAvnControl)projectedButton)));

        Assert.True(Target<Popup>(projectedPopup).InheritsTransform);
        Assert.False(Target<Popup>(projectedPopup).TakesFocusFromNativeControl);
        Assert.True(Target<Popup>(projectedPopup).ShouldUseOverlayLayer);
        Assert.Same(Target<Button>(projectedButton), Target<ContextMenu>(projectedMenu).PlacementTarget);
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
