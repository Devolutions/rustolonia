using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveTextLeftoverComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Line_height_and_scroll_to_line_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBlock(out var projectedBlock));
        Assert.Equal(0, factory.CreateTextBox(out var projectedBox));
        Assert.Equal(0, factory.CreateSelectableTextBlock(out var projectedSelectable));
        Assert.NotNull(projectedBlock);
        Assert.NotNull(projectedBox);
        Assert.NotNull(projectedSelectable);

        Through<IAvnTextBlock>(projectedBlock, block =>
        {
            Assert.Equal(0, block.SetLineHeight(22));
            Assert.Equal(0, block.SetBaselineOffset(4));
        });
        Through<IAvnTextBox>(projectedBox, box =>
            Assert.Equal(0, box.ScrollToLineWithInt32(0)));
        Through<IAvnSelectableTextBlock>(projectedSelectable, selectable =>
            Assert.Equal(0, selectable.SelectAll()));

        Assert.Equal(22, Target<TextBlock>(projectedBlock).LineHeight);
        Assert.Equal(4, Target<TextBlock>(projectedBlock).BaselineOffset);
        Assert.NotNull(Target<TextBox>(projectedBox));
        Assert.NotNull(Target<SelectableTextBlock>(projectedSelectable));
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
