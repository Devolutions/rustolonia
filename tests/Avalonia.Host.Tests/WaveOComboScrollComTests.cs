using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveOComboScrollComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Combo_box_text_and_scroll_offset_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateComboBox(out var projectedCombo));
        Assert.Equal(0, factory.CreateScrollViewer(out var projectedScroll));
        Assert.NotNull(projectedCombo);
        Assert.NotNull(projectedScroll);

        Through<IAvnComboBox>(projectedCombo, combo =>
        {
            Assert.Equal(0, combo.SetIsEditable(1));
            Assert.Equal(0, combo.SetText("typed"));
            Assert.Equal(0, combo.GetText(out var text));
            Assert.Equal("typed", text);
            Assert.Equal(0, combo.Clear());
        });
        Through<IAvnScrollViewer>(projectedScroll, scroll =>
        {
            Assert.Equal(0, scroll.SetOffset(new AvnVector { X = 12, Y = 24 }));
            Assert.Equal(0, scroll.GetOffset(out _));
            Assert.Equal(0, scroll.GetExtent(out _));
            Assert.Equal(0, scroll.GetViewport(out _));
        });

        var combo = Target<ComboBox>(projectedCombo);
        Assert.True(combo.IsEditable);
        Assert.Equal("typed", combo.Text);

        Assert.NotNull(Target<ScrollViewer>(projectedScroll));
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
