using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveNTextBoxComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Text_box_remainder_reaches_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBox(out var projected));
        Assert.Equal(0, factory.CreateMaskedTextBox(out var projectedMask));
        Assert.NotNull(projected);
        Assert.NotNull(projectedMask);

        Through<IAvnTextBox>(projected, box =>
        {
            Assert.Equal(0, box.SetText("hello world"));
            Assert.Equal(0, box.SetTextAlignment((int)TextAlignment.Center));
            Assert.Equal(0, box.SetUseFloatingPlaceholder(1));
            Assert.Equal(0, box.SetClearSelectionOnLostFocus(0));
            Assert.Equal(0, box.SelectAll());
            Assert.Equal(0, box.GetSelectedText(out var selected));
            Assert.Equal("hello world", selected);
        });
        Through<IAvnMaskedTextBox>(projectedMask, mask =>
        {
            Assert.Equal(0, mask.SetMask("000-00-0000"));
            Assert.Equal(0, mask.GetMaskCompleted(out _));
        });

        var value = Target<TextBox>(projected);
        Assert.Equal(TextAlignment.Center, value.TextAlignment);
        Assert.True(value.UseFloatingPlaceholder);
        Assert.False(value.ClearSelectionOnLostFocus);
        Assert.Equal("hello world", value.Text);

        Assert.Equal("000-00-0000", Target<MaskedTextBox>(projectedMask).Mask);
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
