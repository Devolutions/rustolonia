using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU30ObjectListsComTests
{
    [Fact]
    public void Command_bar_primary_commands_cross_as_a_live_control_list()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCommandBar(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCommandBar>(projected);

        Assert.Equal(0, wrapper.GetPrimaryCommands(out var list));
        Assert.NotNull(list);
        Assert.Equal(0, list!.GetCount(out var count));
        Assert.Equal(0, count);

        // Adding through the list persists into the bar's collection.
        Assert.Equal(0, factory.CreateCommandBarButton(out var button));
        var buttonWrapper = Assert.IsType<AvnCommandBarButton>(button);
        Assert.Equal(0, list!.Add(buttonWrapper));
        Assert.Equal(0, list.GetCount(out var added));
        Assert.Equal(1, added);

        var value = Assert.IsType<CommandBar>(
            typeof(AvnCommandBar)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Single(value.PrimaryCommands);

        // The read-only views observe the same collection.
        Assert.Equal(0, wrapper.GetVisiblePrimaryCommands(out var visible));
        Assert.NotNull(visible);
        Assert.Equal(0, visible!.GetCount(out var visibleCount));
        Assert.Equal(1, visibleCount);
    }

    [Fact]
    public void Control_tag_crosses_as_a_variant()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnButton>(projected);

        var tag = AvnVariant.FromObject("payload");
        Assert.Equal(0, wrapper.SetTag(tag));
        Assert.Equal(0, wrapper.GetTag(out var read));
        Assert.Equal(AvnVariant.TagUtf16, read.Tag);

        var value = Assert.IsType<Button>(
            typeof(AvnButton)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Equal("payload", value.Tag);
    }

    [Fact]
    public void Popup_overlay_input_pass_through_crosses_as_a_control()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePopup(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnPopup>(projected);

        Assert.Equal(0, wrapper.GetOverlayInputPassThroughElement(out var read));
        Assert.Null(read);
    }
}
