using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the wave C layout panels: <see cref="WrapPanel"/>, <see cref="UniformGrid"/>,
/// <see cref="RelativePanel"/> (bool attached Align*WithPanel only), <see cref="Viewbox"/>,
/// <see cref="FlexPanel"/>, <see cref="Thumb"/> and <see cref="GridSplitter"/>. Every assertion
/// goes through a real CCW/RCW round trip and then reads the Avalonia object.
/// </summary>
public unsafe class WaveCControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Wrap_panel_spacing_orientation_and_item_size_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWrapPanel(out var projected));
        Assert.NotNull(projected);

        Through<IAvnWrapPanel>(projected, panel =>
        {
            Assert.Equal(0, panel.SetItemSpacing(8));
            Assert.Equal(0, panel.SetLineSpacing(4));
            Assert.Equal(0, panel.SetOrientation((int)Orientation.Vertical));
            Assert.Equal(0, panel.SetItemsAlignment((int)WrapPanelItemsAlignment.Center));
            Assert.Equal(0, panel.SetItemWidth(120));
            Assert.Equal(0, panel.SetItemHeight(32));

            Assert.Equal(0, panel.GetItemSpacing(out var itemSpacing));
            Assert.Equal(8, itemSpacing);
            Assert.Equal(0, panel.GetOrientation(out var orientation));
            Assert.Equal((int)Orientation.Vertical, orientation);
        });

        var value = Target<WrapPanel>(projected);
        Assert.Equal(8, value.ItemSpacing);
        Assert.Equal(4, value.LineSpacing);
        Assert.Equal(Orientation.Vertical, value.Orientation);
        Assert.Equal(WrapPanelItemsAlignment.Center, value.ItemsAlignment);
        Assert.Equal(120, value.ItemWidth);
        Assert.Equal(32, value.ItemHeight);
    }

    [Fact]
    public void Uniform_grid_tracks_and_spacing_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateUniformGrid(out var projected));
        Assert.NotNull(projected);

        Through<IAvnUniformGrid>(projected, grid =>
        {
            Assert.Equal(0, grid.SetRows(3));
            Assert.Equal(0, grid.SetColumns(4));
            Assert.Equal(0, grid.SetFirstColumn(1));
            Assert.Equal(0, grid.SetRowSpacing(6));
            Assert.Equal(0, grid.SetColumnSpacing(8));
        });

        var value = Target<UniformGrid>(projected);
        Assert.Equal(3, value.Rows);
        Assert.Equal(4, value.Columns);
        Assert.Equal(1, value.FirstColumn);
        Assert.Equal(6, value.RowSpacing);
        Assert.Equal(8, value.ColumnSpacing);
    }

    [Fact]
    public void Relative_panel_align_with_panel_bools_cross_as_attached_properties()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);
        Assert.Equal(0, factory.GetRelativePanelStatics(out var projectedStatics));
        Assert.NotNull(projectedStatics);
        var button = Target<Button>(projected);

        Through<IAvnRelativePanelStatics>(projectedStatics, statics =>
        {
            Through<IAvnControl>(projected, target =>
            {
                Assert.Equal(0, statics.GetAlignLeftWithPanel(target, out var initial));
                Assert.Equal(0, initial);

                Assert.Equal(0, statics.SetAlignLeftWithPanel(target, 1));
                Assert.Equal(0, statics.SetAlignTopWithPanel(target, 1));
                Assert.Equal(0, statics.GetAlignLeftWithPanel(target, out var left));
                Assert.Equal(1, left);
            });
        });

        Assert.True(RelativePanel.GetAlignLeftWithPanel(button));
        Assert.True(RelativePanel.GetAlignTopWithPanel(button));
        Assert.False(RelativePanel.GetAlignRightWithPanel(button));
    }

    [Fact]
    public void Viewbox_child_and_stretch_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateViewbox(out var projected));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedChild));
        Assert.NotNull(projected);
        Assert.NotNull(projectedChild);

        Through<IAvnViewbox>(projected, viewbox =>
        {
            Assert.Equal(0, viewbox.GetChild(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, projectedChild.SetText("scaled"));
            Assert.Equal(0, viewbox.SetChild((IAvnControl)projectedChild));
            Assert.Equal(0, viewbox.SetStretch((int)Stretch.UniformToFill));
            Assert.Equal(0, viewbox.SetStretchDirection((int)StretchDirection.DownOnly));

            Assert.Equal(0, viewbox.GetChild(out var child));
            Assert.NotNull(child);
        });

        var value = Target<Viewbox>(projected);
        Assert.Equal("scaled", Assert.IsType<TextBlock>(value.Child).Text);
        Assert.Equal(Stretch.UniformToFill, value.Stretch);
        Assert.Equal(StretchDirection.DownOnly, value.StretchDirection);
    }

    [Fact]
    public void Flex_panel_direction_and_alignment_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateFlexPanel(out var projected));
        Assert.NotNull(projected);

        Through<IAvnFlexPanel>(projected, panel =>
        {
            Assert.Equal(0, panel.SetDirection((int)FlexDirection.Column));
            Assert.Equal(0, panel.SetJustifyContent((int)FlexJustifyContent.Center));
            Assert.Equal(0, panel.SetAlignItems((int)FlexAlignItems.FlexStart));
            Assert.Equal(0, panel.SetWrap((int)FlexWrap.Wrap));
            Assert.Equal(0, panel.SetColumnSpacing(10));
            Assert.Equal(0, panel.SetRowSpacing(6));
        });

        var value = Target<FlexPanel>(projected);
        Assert.Equal(FlexDirection.Column, value.Direction);
        Assert.Equal(FlexJustifyContent.Center, value.JustifyContent);
        Assert.Equal(FlexAlignItems.FlexStart, value.AlignItems);
        Assert.Equal(FlexWrap.Wrap, value.Wrap);
        Assert.Equal(10, value.ColumnSpacing);
        Assert.Equal(6, value.RowSpacing);
    }

    [Fact]
    public void Grid_splitter_resize_members_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGridSplitter(out var projected));
        Assert.NotNull(projected);

        Through<IAvnGridSplitter>(projected, splitter =>
        {
            Assert.Equal(0, splitter.SetResizeDirection((int)GridResizeDirection.Rows));
            Assert.Equal(0, splitter.SetResizeBehavior((int)GridResizeBehavior.PreviousAndNext));
            Assert.Equal(0, splitter.SetShowsPreview(1));
            Assert.Equal(0, splitter.SetKeyboardIncrement(20));
            Assert.Equal(0, splitter.SetDragIncrement(4));
        });

        var value = Target<GridSplitter>(projected);
        Assert.Equal(GridResizeDirection.Rows, value.ResizeDirection);
        Assert.Equal(GridResizeBehavior.PreviousAndNext, value.ResizeBehavior);
        Assert.True(value.ShowsPreview);
        Assert.Equal(20, value.KeyboardIncrement);
        Assert.Equal(4, value.DragIncrement);
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
