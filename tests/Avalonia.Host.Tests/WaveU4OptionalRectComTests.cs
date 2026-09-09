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

public unsafe class WaveU4OptionalRectComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Placement_rect_round_trips_a_value_and_clears_to_null()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePopup(out var projected));
        Assert.NotNull(projected);

        Through<IAvnPopup>(projected, popup =>
        {
            var rect = AvnOptionalRect.FromAvalonia(new Rect(1, 2, 3, 4));
            Assert.Equal(0, popup.SetPlacementRect(rect));
            Assert.Equal(0, popup.GetPlacementRect(out var value));
            Assert.Equal(1, value.HasValue);
            Assert.Equal(1, value.Value.X);
            Assert.Equal(2, value.Value.Y);
            Assert.Equal(3, value.Value.Width);
            Assert.Equal(4, value.Value.Height);

            Assert.Equal(0, popup.SetPlacementRect(default));
            Assert.Equal(0, popup.GetPlacementRect(out value));
            Assert.Equal(0, value.HasValue);
        });
        Assert.Null(Target<Popup>(projected).PlacementRect);
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
