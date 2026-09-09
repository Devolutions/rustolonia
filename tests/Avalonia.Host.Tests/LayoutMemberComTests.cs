using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Layout;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the layout members published on the widened <c>IAvnControl</c>,
/// <c>IAvnDecorator</c>, <c>IAvnStyledElement</c> and <c>IAvnWindow</c> vtables. Every
/// assertion goes through a real CCW/RCW round trip so <c>AvnThickness</c> crosses the
/// generated marshalling stubs by value rather than being read off the managed wrapper.
/// </summary>
public unsafe class LayoutMemberComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Control_layout_members_round_trip_and_reach_the_avalonia_object()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);
        var button = Target<Button>(projected);

        Through<IAvnButton>(projected, control =>
        {
            Assert.Equal(0, control.SetMargin(new AvnThickness { Left = 1, Top = 2.5, Right = -3, Bottom = 4.25 }));
            Assert.Equal(0, control.GetMargin(out var margin));
            Assert.Equal(new Thickness(1, 2.5, -3, 4.25), margin.ToAvalonia());

            Assert.Equal(0, control.SetIsVisible(0));
            Assert.Equal(0, control.GetIsVisible(out var isVisible));
            Assert.Equal(0, isVisible);

            Assert.Equal(0, control.SetOpacity(0.25));
            Assert.Equal(0, control.GetOpacity(out var opacity));
            Assert.Equal(0.25, opacity);

            Assert.Equal(0, control.SetHorizontalAlignment((int)HorizontalAlignment.Right));
            Assert.Equal(0, control.GetHorizontalAlignment(out var horizontal));
            Assert.Equal((int)HorizontalAlignment.Right, horizontal);

            Assert.Equal(0, control.SetVerticalAlignment((int)VerticalAlignment.Bottom));
            Assert.Equal(0, control.GetVerticalAlignment(out var vertical));
            Assert.Equal((int)VerticalAlignment.Bottom, vertical);

            Assert.Equal(0, control.SetMinWidth(10));
            Assert.Equal(0, control.SetMinHeight(20));
            Assert.Equal(0, control.SetMaxWidth(300));
            Assert.Equal(0, control.SetMaxHeight(400));
            Assert.Equal(0, control.GetMinWidth(out var minWidth));
            Assert.Equal(0, control.GetMinHeight(out var minHeight));
            Assert.Equal(0, control.GetMaxWidth(out var maxWidth));
            Assert.Equal(0, control.GetMaxHeight(out var maxHeight));
            Assert.Equal((10d, 20d, 300d, 400d), (minWidth, minHeight, maxWidth, maxHeight));
        });

        // The projection must have written through to the real control, not to wrapper state.
        Assert.Equal(new Thickness(1, 2.5, -3, 4.25), button.Margin);
        Assert.False(button.IsVisible);
        Assert.Equal(0.25, button.Opacity);
        Assert.Equal(HorizontalAlignment.Right, button.HorizontalAlignment);
        Assert.Equal(VerticalAlignment.Bottom, button.VerticalAlignment);
        Assert.Equal(300d, button.MaxWidth);
    }

    [Fact]
    public void Styled_element_name_round_trips_before_styles_are_applied()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBlock(out var projected));
        Assert.NotNull(projected);

        Through<IAvnTextBlock>(projected, element =>
        {
            Assert.Equal(0, element.GetName(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, element.SetName("readout"));
            Assert.Equal(0, element.GetName(out var name));
            Assert.Equal("readout", name);
        });

        Assert.Equal("readout", Target<TextBlock>(projected).Name);
    }

    [Fact]
    public void Decorator_padding_round_trips_by_value()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);

        Through<IAvnBorder>(projected, border =>
        {
            Assert.Equal(0, border.SetPadding(AvnThickness.FromAvalonia(new Thickness(8))));
            Assert.Equal(0, border.GetPadding(out var padding));
            Assert.Equal(new Thickness(8), padding.ToAvalonia());
        });

        Assert.Equal(new Thickness(8), Target<Border>(projected).Padding);
    }

    [Fact]
    public void Window_exposes_can_resize_window_state_and_inherited_layout_members()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindow(out var projected));
        Assert.NotNull(projected);

        Through<IAvnWindow>(projected, window =>
        {
            Assert.Equal(0, window.GetCanResize(out var initial));
            Assert.Equal(1, initial);

            Assert.Equal(0, window.SetCanResize(0));
            Assert.Equal(0, window.GetCanResize(out var canResize));
            Assert.Equal(0, canResize);

            Assert.Equal(0, window.GetWindowState(out var windowState));
            Assert.Equal((int)WindowState.Normal, windowState);

            // Width/Height/Margin are inherited from the widened IAvnControl slots.
            Assert.Equal(0, window.SetWidth(640));
            Assert.Equal(0, window.SetHeight(480));
            Assert.Equal(0, window.SetMargin(AvnThickness.FromAvalonia(new Thickness(2, 4))));
            Assert.Equal(0, window.GetMargin(out var margin));
            Assert.Equal(new Thickness(2, 4), margin.ToAvalonia());
        });

        var value = Target<Window>(projected);
        Assert.False(value.CanResize);
        Assert.Equal(640d, value.Width);
        Assert.Equal(480d, value.Height);
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
