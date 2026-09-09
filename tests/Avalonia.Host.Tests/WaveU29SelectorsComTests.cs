using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU29SelectorsComTests
{
    [Fact]
    public void Item_selector_round_trips_through_the_ccw()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);

        var selector = new ItemSelector();
        Assert.Equal(0, wrapper.SetItemSelector(selector));
        Assert.Equal(0, wrapper.GetItemSelector(out var read));
        // Reading back wraps the foreign interface in the host's AvnItemSelector.
        Assert.IsType<AvnItemSelector>(read);

        // The host wrapper converts a foreign interface back into the delegate and
        // invokes it, so a foreign CCW really selects the item text.
        var value = Assert.IsType<AutoCompleteBox>(
            typeof(AvnAutoCompleteBox)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var managed = value.ItemSelector;
        Assert.NotNull(managed);
        var item = new object();
        Assert.Equal($"Search|{item}", managed!("Search", item));

        // Null clears the selector.
        Assert.Equal(0, wrapper.SetItemSelector(null));
        Assert.Equal(0, wrapper.GetItemSelector(out var cleared));
        Assert.Null(cleared);
    }

    [Fact]
    public void Text_selector_round_trips_through_the_ccw()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);

        var selector = new TextSelector();
        Assert.Equal(0, wrapper.SetTextSelector(selector));
        Assert.Equal(0, wrapper.GetTextSelector(out var read));
        // Reading back wraps the foreign interface in the host's AvnTextSelector.
        Assert.IsType<AvnTextSelector>(read);

        var value = Assert.IsType<AutoCompleteBox>(
            typeof(AvnAutoCompleteBox)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var managed = value.TextSelector;
        Assert.NotNull(managed);
        Assert.Equal("Search|Item", managed!("Search", "Item"));

        Assert.Equal(0, wrapper.SetTextSelector(null));
        Assert.Equal(0, wrapper.GetTextSelector(out var cleared));
        Assert.Null(cleared);
    }

    private sealed class ItemSelector : IAvnItemSelector
    {
        public int Invoke(string? search, AvnVariant item, out string? text)
        {
            text = $"{search}|{item.ToObject()}";
            return 0;
        }
    }

    private sealed class TextSelector : IAvnTextSelector
    {
        public int Invoke(string? search, string? item, out string? text)
        {
            text = $"{search}|{item}";
            return 0;
        }
    }
}
