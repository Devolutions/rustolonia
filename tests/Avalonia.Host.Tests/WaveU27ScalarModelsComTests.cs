using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.Styling;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU27ScalarModelsComTests
{
    [Fact]
    public void Button_hot_key_crosses_as_a_gesture_string()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnButton>(projected);

        Assert.Equal(0, wrapper.SetHotKey("Ctrl+S"));
        var value = Assert.IsType<Button>(
            typeof(AvnButton)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.NotNull(value.HotKey);
        Assert.Equal(0, wrapper.GetHotKey(out var read));
        Assert.Equal("Ctrl+S", read);

        Assert.Equal(0, wrapper.SetHotKey(null));
        Assert.Equal(0, wrapper.GetHotKey(out var cleared));
        Assert.Null(cleared);
    }

    [Fact]
    public void Menu_item_input_gesture_crosses_as_a_gesture_string()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMenuItem(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnMenuItem>(projected);

        Assert.Equal(0, wrapper.SetInputGesture("F5"));
        Assert.Equal(0, wrapper.GetInputGesture(out var read));
        Assert.Equal("F5", read);
    }

    [Fact]
    public void Border_box_shadow_crosses_as_a_shadow_list_string()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnBorder>(projected);

        Assert.Equal(0, wrapper.SetBoxShadow("2 4 8 rgb(255,12,0)"));
        var value = Assert.IsType<Border>(
            typeof(AvnBorder)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Equal(1, value.BoxShadow.Count);
        Assert.Equal(2, value.BoxShadow[0].OffsetX);
        Assert.Equal(4, value.BoxShadow[0].OffsetY);
        Assert.Equal(0, wrapper.GetBoxShadow(out var read));
        Assert.Contains("ff0c00", read!.ToLowerInvariant());

        // A comma-separated pair crosses as two shadows.
        Assert.Equal(0, wrapper.SetBoxShadow("0 1 2 rgb(0,12,255), 3 4 5 rgb(0,255,12)"));
        Assert.Equal(0, wrapper.GetBoxShadow(out var pair));
        Assert.Contains(',', pair!);

        // "none" clears the shadows.
        Assert.Equal(0, wrapper.SetBoxShadow("none"));
        Assert.Equal(0, wrapper.GetBoxShadow(out var cleared));
        Assert.Equal("none", cleared);
    }

    [Fact]
    public void Tab_item_tab_strip_placement_crosses_as_a_dock_name()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTabItem(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTabItem>(projected);

        // TabStripPlacement is read-only on the CLR (internal setter), so the ABI
        // publishes a getter only.
        Assert.Equal(0, wrapper.GetTabStripPlacement(out var read));
        Assert.Null(read);
    }

    [Fact]
    public void Theme_variant_scope_requested_variant_crosses_as_a_name()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateThemeVariantScope(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnThemeVariantScope>(projected);

        Assert.Equal(0, wrapper.SetRequestedThemeVariant("Dark"));
        var value = Assert.IsType<ThemeVariantScope>(
            typeof(AvnThemeVariantScope)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Equal(ThemeVariant.Dark, value.RequestedThemeVariant);
        Assert.Equal(0, wrapper.GetRequestedThemeVariant(out var read));
        Assert.Equal("Dark", read);

        // A null string clears the request.
        Assert.Equal(0, wrapper.SetRequestedThemeVariant(null));
        Assert.Equal(0, wrapper.GetRequestedThemeVariant(out var cleared));
        Assert.Null(cleared);
    }
}
