using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU18VariantSurfaceComTests
{
    [Fact]
    public void Items_source_round_trips_through_the_variant_list_adapter()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnListBox>(projected);

        Assert.Equal(0, wrapper.GetItemsSource(out var none));
        Assert.Null(none);

        Assert.Equal(0, wrapper.SetItemsSource(
            AvnObjectList.FromManaged((System.Collections.IEnumerable)new System.Collections.Generic.List<object?> { "alpha", "beta" })));
        Assert.Equal(0, wrapper.GetItemsSource(out var list));
        Assert.NotNull(list);
        Assert.Equal(0, list!.GetCount(out var count));
        Assert.Equal(2, count);
        Assert.Equal(0, list.GetAt(1, out var item));
        Assert.Equal("beta", (string?)item.ToObject());

        // Mutation materializes the shadow; reading it back persists the list.
        Assert.Equal(0, list.Add(AvnVariant.FromObject("gamma")));
        Assert.Equal(0, list.GetCount(out count));
        Assert.Equal(3, count);

        Assert.Equal(0, wrapper.SetItemsSource(null));
        Assert.Equal(0, wrapper.GetItemsSource(out none));
        Assert.Null(none);
    }

    [Fact]
    public void Selected_item_crosses_as_a_variant()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnListBox>(projected);

        Assert.Equal(0, wrapper.GetSelectedItem(out var none));
        Assert.Equal(AvnVariant.TagNone, none.Tag);

        Assert.Equal(0, wrapper.SetSelectedItem(AvnVariant.FromObject("chosen")));
        Assert.Equal(0, wrapper.GetSelectedItem(out var selected));
        Assert.Equal(AvnVariant.TagUtf16, selected.Tag);
        Assert.Equal("chosen", (string?)selected.ToObject());
    }

    [Fact]
    public void Tab_control_selected_content_crosses_as_a_variant()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTabControl(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTabControl>(projected);

        Assert.Equal(0, wrapper.GetSelectedContent(out var content));
        Assert.Equal(AvnVariant.TagNone, content.Tag);
    }

    [Fact]
    public void Notification_manager_closes_all()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindowNotificationManager(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnWindowNotificationManager>(projected);
        Assert.Equal(0, wrapper.CloseAll());
    }
}
