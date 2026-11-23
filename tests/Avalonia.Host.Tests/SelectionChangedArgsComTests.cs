using System.Reflection;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Selection-changed payloads can carry null entries: Avalonia reports the removed
/// item of a pending index that never matched an item as null. A null entry at a
/// valid index crosses as the empty variant; only an out-of-range index is invalid.
/// </summary>
public class SelectionChangedArgsComTests
{
    [Fact]
    public void Null_entry_at_a_valid_index_crosses_as_the_empty_variant()
    {
        var args = new AvnSelectingItemsControlSelectionChangedArgs(
            new System.Collections.IEnumerable?[] { new object?[] { "a", null }, null });

        Assert.Equal(HResults.S_OK, args.GetAddedItemsCount(out var count));
        Assert.Equal(2, count);
        Assert.Equal(HResults.S_OK, args.GetAddedItemsAt(1, out var empty));
        Assert.Equal(0, empty.Tag);
        Assert.Equal(HResults.S_OK, args.GetAddedItemsAt(0, out var text));
        Assert.Equal(AvnVariant.TagUtf16, text.Tag);

        Assert.Equal(HResults.E_INVALIDARG, args.GetAddedItemsAt(2, out _));
        Assert.Equal(HResults.E_INVALIDARG, args.GetAddedItemsAt(-1, out _));
        Assert.Equal(HResults.S_OK, args.GetRemovedItemsCount(out var removed));
        Assert.Equal(0, removed);
        Assert.Equal(HResults.E_INVALIDARG, args.GetRemovedItemsAt(0, out _));
    }

    [Fact]
    public void Adding_items_after_an_unmatched_pending_index_reaches_the_handler()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateComboBox(out var projected));
        var wrapper = Assert.IsType<AvnComboBox>(projected);
        var combo = Assert.IsType<ComboBox>(
            typeof(AvnComboBox)
                .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var handler = new ReadAllHandler();

        Assert.Equal(0, wrapper.SetSelectedIndex(2));
        Assert.Equal(0, wrapper.AdviseSelectionChanged(handler, out _));

        combo.Items.Add(new ComboBoxItem { Content = "Red" });
        combo.Items.Add(new ComboBoxItem { Content = "Green" });
        combo.Items.Add(new ComboBoxItem { Content = "Blue" });

        Assert.Equal(-1, combo.SelectedIndex);
        Assert.Equal(HResults.S_OK, handler.LastResult);
    }

    // Reads every entry and fails the event on any accessor error, as the Rust
    // payload decoder does.
    private sealed class ReadAllHandler : IAvnSelectingItemsControlSelectionChangedHandler
    {
        public int LastResult { get; private set; } = HResults.S_OK;

        public int Invoke(IAvnSelectingItemsControlSelectionChangedArgs args)
        {
            LastResult = ReadAll(args);
            return LastResult;
        }

        private static int ReadAll(IAvnSelectingItemsControlSelectionChangedArgs args)
        {
            var hr = args.GetAddedItemsCount(out var added);
            for (var i = 0; hr >= 0 && i < added; i++)
                hr = args.GetAddedItemsAt(i, out _);
            if (hr < 0)
                return hr;
            hr = args.GetRemovedItemsCount(out var removed);
            for (var i = 0; hr >= 0 && i < removed; i++)
                hr = args.GetRemovedItemsAt(i, out _);
            return hr;
        }
    }
}
