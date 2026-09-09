using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the wave D button family, <see cref="ContextMenu"/> and <see cref="MenuFlyout"/>.
/// </summary>
public unsafe class WaveDControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Repeat_button_delay_and_interval_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateRepeatButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnRepeatButton>(projected, button =>
        {
            Assert.Equal(0, button.SetDelay(400));
            Assert.Equal(0, button.SetInterval(50));
        });

        var value = Target<RepeatButton>(projected);
        Assert.Equal(400, value.Delay);
        Assert.Equal(50, value.Interval);
    }

    [Fact]
    public void Hyperlink_navigate_uri_crosses_as_a_string()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateHyperlinkButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnHyperlinkButton>(projected, button =>
        {
            Assert.Equal(0, button.GetNavigateUri(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, button.SetNavigateUri("https://avaloniaui.net"));
            Assert.Equal(0, button.GetNavigateUri(out var uri));
            Assert.Equal("https://avaloniaui.net", uri);

            Assert.Equal(0, button.SetIsVisited(1));
            Assert.Equal(0, button.SetNavigateUri("docs/readme.md"));
            Assert.Equal(0, button.GetNavigateUri(out var relative));
            Assert.Equal("docs/readme.md", relative);

            // Uri.TryCreate(RelativeOrAbsolute) accepts almost any non-empty spelling as a
            // relative URI, so there is no "malformed" case to reject besides empty/null,
            // which clear the property.
            Assert.Equal(0, button.SetNavigateUri(string.Empty));
            Assert.Equal(0, button.GetNavigateUri(out var cleared));
            Assert.Null(cleared);
        });

        var value = Target<HyperlinkButton>(projected);
        Assert.Null(value.NavigateUri);
        Assert.True(value.IsVisited);
    }

    [Fact]
    public void Toggle_split_button_checked_state_reaches_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateToggleSplitButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnToggleSplitButton>(projected, button =>
        {
            Assert.Equal(0, button.SetIsChecked(1));
            Assert.Equal(0, button.GetIsChecked(out var isChecked));
            Assert.Equal(1, isChecked);
        });

        Assert.True(Target<ToggleSplitButton>(projected).IsChecked);
    }

    [Fact]
    public void Context_menu_placement_and_offsets_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateContextMenu(out var projected));
        Assert.NotNull(projected);

        Through<IAvnContextMenu>(projected, menu =>
        {
            Assert.Equal(0, menu.SetHorizontalOffset(12));
            Assert.Equal(0, menu.SetVerticalOffset(-4));
            Assert.Equal(0, menu.SetPlacement((int)PlacementMode.Bottom));
            Assert.Equal(0, menu.SetWindowManagerAddShadowHint(1));
        });

        var value = Target<ContextMenu>(projected);
        Assert.Equal(12, value.HorizontalOffset);
        Assert.Equal(-4, value.VerticalOffset);
        Assert.Equal(PlacementMode.Bottom, value.Placement);
        Assert.True(value.WindowManagerAddShadowHint);
    }

    [Fact]
    public void Menu_flyout_items_are_the_same_item_list_as_items_control()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMenuFlyout(out var projected));
        Assert.Equal(0, factory.CreateMenuItem(out var projectedItem));
        Assert.NotNull(projected);
        Assert.NotNull(projectedItem);

        Through<IAvnMenuFlyout>(projected, flyout =>
        {
            Assert.Equal(0, flyout.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedItem));
            Assert.Equal(0, items.GetCount(out var count));
            Assert.Equal(1, count);
        });

        Assert.Single(Target<MenuFlyout>(projected).Items);
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
