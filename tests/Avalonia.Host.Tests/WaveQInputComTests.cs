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

public unsafe class WaveQInputComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Leaf_input_scalars_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projectedBox));
        Assert.Equal(0, factory.CreateCalendar(out var projectedCalendar));
        Assert.Equal(0, factory.CreateNumericUpDown(out var projectedNumeric));
        Assert.NotNull(projectedBox);
        Assert.NotNull(projectedCalendar);
        Assert.NotNull(projectedNumeric);

        Through<IAvnAutoCompleteBox>(projectedBox, box =>
        {
            Assert.Equal(0, box.SetText("av"));
            Assert.Equal(0, box.SetMaxLength(32));
            Assert.Equal(0, box.SetClearSelectionOnLostFocus(0));
            Assert.Equal(0, box.GetSearchText(out _));
        });
        Through<IAvnCalendar>(projectedCalendar, calendar =>
            Assert.Equal(0, calendar.SetIsWeekNumberVisible(1)));
        Through<IAvnNumericUpDown>(projectedNumeric, numeric =>
            Assert.Equal(0, numeric.SetTextAlignment((int)TextAlignment.Right)));

        Assert.Equal("av", Target<AutoCompleteBox>(projectedBox).Text);
        Assert.Equal(32, Target<AutoCompleteBox>(projectedBox).MaxLength);
        Assert.True(Target<Calendar>(projectedCalendar).IsWeekNumberVisible);
        Assert.Equal(TextAlignment.Right, Target<NumericUpDown>(projectedNumeric).TextAlignment);
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
