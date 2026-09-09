using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU24FilterComTests
{
    [Fact]
    public void Item_filter_round_trips_and_invokes()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);

        Assert.Equal(0, wrapper.GetItemFilter(out var none));
        Assert.Null(none);

        Assert.Equal(0, wrapper.SetItemFilter(new ItemFilterImpl()));
        Assert.Equal(0, wrapper.GetItemFilter(out var read));
        Assert.NotNull(read);
        Assert.Equal(0, read!.Invoke("search", AvnVariant.FromObject("item"), out var result));
        Assert.Equal(1, result);
    }

    [Fact]
    public void Text_filter_round_trips_and_invokes()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);

        Assert.Equal(0, wrapper.SetTextFilter(new TextFilterImpl()));
        Assert.Equal(0, wrapper.GetTextFilter(out var read));
        Assert.NotNull(read);
        Assert.Equal(0, read!.Invoke("search", "item", out var result));
        Assert.Equal(1, result);

        // The managed side sees a delegate wrapping the CCW round trip.
        var value = Assert.IsType<AutoCompleteBox>(
            typeof(AvnAutoCompleteBox)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.NotNull(value.TextFilter);
        Assert.True(value.TextFilter!("search", "item"));
    }

    private sealed class ItemFilterImpl : IAvnItemFilter
    {
        public int Invoke(string? search, AvnVariant item, out int result)
        {
            result = 1;
            return 0;
        }
    }

    private sealed class TextFilterImpl : IAvnTextFilter
    {
        public int Invoke(string? search, string? item, out int result)
        {
            result = 1;
            return 0;
        }
    }
}
