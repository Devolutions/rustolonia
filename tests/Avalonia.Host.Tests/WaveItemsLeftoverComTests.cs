using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveItemsLeftoverComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Item_count_and_auto_scroll_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);

        Through<IAvnListBox>(projected, list =>
        {
            Assert.Equal(0, list.SetAutoScrollToSelectedItem(0));
            Assert.Equal(0, list.SetIsTextSearchEnabled(0));
            Assert.Equal(0, list.SetWrapSelection(1));
            Assert.Equal(0, list.GetItemCount(out var count));
            Assert.Equal(0, count);
            Assert.Equal(0, list.ScrollIntoViewWithInt32(0));
        });

        var box = Target<ListBox>(projected);
        Assert.False(box.AutoScrollToSelectedItem);
        Assert.False(box.IsTextSearchEnabled);
        Assert.True(box.WrapSelection);
        Assert.Equal(0, box.ItemCount);
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
