using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the wave A controls: <see cref="Image"/>, the tab pair, the tree pair and the
/// <see cref="ToolTip"/> attached properties. Every assertion goes through a real CCW/RCW round
/// trip and then reads the Avalonia object, so a member that only updated wrapper state fails.
/// </summary>
public unsafe class WaveAControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Image_source_crosses_as_a_file_path_and_reaches_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var path = Path.Combine(Path.GetDirectoryName(typeof(WaveAControlComTests).Assembly.Location)!, "logo.png");
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateImage(out var projected));
        Assert.NotNull(projected);

        Through<IAvnImage>(projected, image =>
        {
            // A fresh Image has no source, and that is a null rather than an empty path.
            Assert.Equal(0, image.GetSource(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, image.SetSource(path));
            // The getter hands back the exact string that was set, not the bitmap's type name.
            Assert.Equal(0, image.GetSource(out var source));
            Assert.Equal(path, source);

            Assert.Equal(0, image.SetStretch((int)Stretch.UniformToFill));
            Assert.Equal(0, image.GetStretch(out var stretch));
            Assert.Equal((int)Stretch.UniformToFill, stretch);

            Assert.Equal(0, image.SetStretchDirection((int)StretchDirection.DownOnly));
            Assert.Equal(0, image.GetStretchDirection(out var stretchDirection));
            Assert.Equal((int)StretchDirection.DownOnly, stretchDirection);
        });

        var value = Target<Image>(projected);
        Assert.IsType<Bitmap>(value.Source);
        Assert.Equal(Stretch.UniformToFill, value.Stretch);
        Assert.Equal(StretchDirection.DownOnly, value.StretchDirection);
    }

    [Fact]
    public void Image_source_clears_on_empty_and_reads_null_for_an_image_the_abi_never_set()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateImage(out var projected));
        Assert.NotNull(projected);
        var value = Target<Image>(projected);

        Through<IAvnImage>(projected, image =>
        {
            Assert.Equal(0, image.SetSource("logo.png"));
            Assert.NotNull(value.Source);

            Assert.Equal(0, image.SetSource(null));
            Assert.Null(value.Source);
            Assert.Equal(0, image.GetSource(out var cleared));
            Assert.Null(cleared);
        });

        // An image the managed side installed has no source string to report, so the getter
        // says null even though the control is drawing it. That is the documented limit of
        // projecting Source as a string.
        value.Source = new Bitmap(Stream.Null);
        Through<IAvnImage>(projected, image =>
        {
            Assert.Equal(0, image.GetSource(out var source));
            Assert.Null(source);
        });
    }

    [Fact]
    public void An_unresolvable_image_source_fails_the_call_rather_than_being_guessed_at()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateImage(out var projected));
        Assert.NotNull(projected);

        Through<IAvnImage>(projected, image =>
        {
            Assert.True(image.SetSource("https://example.invalid/logo.png") < 0);
            Assert.Equal(0, image.GetSource(out var source));
            Assert.Null(source);
        });

        Assert.Null(Target<Image>(projected).Source);
    }

    [Fact]
    public void Tab_control_and_tab_item_round_trip_over_their_inherited_selection_members()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTabControl(out var projectedControl));
        Assert.Equal(0, factory.CreateTabItem(out var projectedFirst));
        Assert.Equal(0, factory.CreateTabItem(out var projectedSecond));
        Assert.NotNull(projectedControl);
        Assert.NotNull(projectedFirst);
        Assert.NotNull(projectedSecond);

        Through<IAvnTabControl>(projectedControl, tabControl =>
        {
            Assert.Equal(0, tabControl.SetTabStripPlacement((int)Dock.Bottom));
            Assert.Equal(0, tabControl.GetTabStripPlacement(out var placement));
            Assert.Equal((int)Dock.Bottom, placement);

            Assert.Equal(0, tabControl.SetHorizontalContentAlignment((int)HorizontalAlignment.Right));
            Assert.Equal(0, tabControl.GetHorizontalContentAlignment(out var horizontal));
            Assert.Equal((int)HorizontalAlignment.Right, horizontal);

            // Items and SelectedIndex are inherited from IAvnSelectingItemsControl rather than
            // redeclared on the tab control.
            Assert.Equal(0, tabControl.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedFirst));
            Assert.Equal(0, items.Add((IAvnControl)projectedSecond));
            Assert.Equal(0, tabControl.SetSelectedIndex(1));
            Assert.Equal(0, tabControl.GetSelectedIndex(out var selectedIndex));
            Assert.Equal(1, selectedIndex);
        });

        Through<IAvnTabItem>(projectedFirst, tabItem =>
        {
            // Header comes from IAvnHeaderedContentControl and crosses as a control.
            Assert.Equal(0, factory.CreateTextBlock(out var header));
            Assert.NotNull(header);
            Assert.Equal(0, header.SetText("First"));
            Assert.Equal(0, tabItem.SetHeader(header));
            Assert.Equal(0, tabItem.GetHeader(out var readBack));
            Assert.NotNull(readBack);

            Assert.Equal(0, tabItem.GetIsSelected(out var isSelected));
            Assert.Equal(0, isSelected);
        });

        var control = Target<TabControl>(projectedControl);
        Assert.Equal(Dock.Bottom, control.TabStripPlacement);
        Assert.Equal(HorizontalAlignment.Right, control.HorizontalContentAlignment);
        Assert.Equal(1, control.SelectedIndex);
        Assert.Equal("First", Assert.IsType<TextBlock>(Target<TabItem>(projectedFirst).Header).Text);
    }

    [Fact]
    public void Tree_view_items_nest_expand_and_expose_their_level()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTreeView(out var projectedView));
        Assert.Equal(0, factory.CreateTreeViewItem(out var projectedRoot));
        Assert.Equal(0, factory.CreateTreeViewItem(out var projectedChild));
        Assert.NotNull(projectedView);
        Assert.NotNull(projectedRoot);
        Assert.NotNull(projectedChild);

        Through<IAvnTreeViewItem>(projectedRoot, root =>
        {
            Assert.Equal(0, factory.CreateTextBlock(out var header));
            Assert.NotNull(header);
            Assert.Equal(0, header.SetText("Root"));
            Assert.Equal(0, root.SetHeader(header));

            // A TreeViewItem is an ItemsControl, so children go into the inherited Items slot.
            Assert.Equal(0, root.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedChild));

            Assert.Equal(0, root.SetIsExpanded(1));
            Assert.Equal(0, root.GetIsExpanded(out var isExpanded));
            Assert.Equal(1, isExpanded);

            // Level is maintained by the control, so it reads back without ever being written.
            Assert.Equal(0, root.GetLevel(out var level));
            Assert.Equal(0, level);
        });

        Through<IAvnTreeView>(projectedView, treeView =>
        {
            Assert.Equal(0, treeView.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedRoot));

            Assert.Equal(0, treeView.SetSelectionMode((int)SelectionMode.Multiple));
            Assert.Equal(0, treeView.GetSelectionMode(out var selectionMode));
            Assert.Equal((int)SelectionMode.Multiple, selectionMode);

            Assert.Equal(0, treeView.SetAutoScrollToSelectedItem(0));
            Assert.Equal(0, treeView.GetAutoScrollToSelectedItem(out var autoScroll));
            Assert.Equal(0, autoScroll);

            // The subtree commands take a projected TreeViewItem, so the ABI unwraps it back to
            // the Avalonia object instead of crossing an opaque handle.
            Assert.Equal(0, treeView.CollapseSubTreeWithTreeViewItem((IAvnTreeViewItem)projectedRoot));
            Assert.Equal(0, treeView.UnselectAll());
        });

        var view = Target<TreeView>(projectedView);
        Assert.Equal(SelectionMode.Multiple, view.SelectionMode);
        Assert.False(view.AutoScrollToSelectedItem);
        var item = Target<TreeViewItem>(projectedRoot);
        Assert.False(item.IsExpanded);
        Assert.Equal("Root", Assert.IsType<TextBlock>(item.Header).Text);
    }

    [Fact]
    public void Tree_view_item_expand_and_collapse_events_bridge_to_the_abi()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTreeViewItem(out var projected));
        Assert.NotNull(projected);
        var wrapper = Assert.IsType<AvnTreeViewItem>(projected);
        var value = Target<TreeViewItem>(projected);
        var expanded = new ExpandedHandler();
        var collapsed = new CollapsedHandler();

        Assert.Equal(0, wrapper.AdviseExpanded(expanded, out var expandedId));
        Assert.Equal(0, wrapper.AdviseCollapsed(collapsed, out var collapsedId));

        value.IsExpanded = true;
        Assert.Equal(1, expanded.CallCount);
        value.IsExpanded = false;
        Assert.Equal(1, collapsed.CallCount);

        Assert.Equal(0, wrapper.UnadviseExpanded(expandedId));
        Assert.Equal(0, wrapper.UnadviseCollapsed(collapsedId));
        value.IsExpanded = true;
        Assert.Equal(1, expanded.CallCount);
        Assert.True(wrapper.UnadviseExpanded(expandedId) < 0);
    }

    private sealed class ExpandedHandler : IAvnTreeViewItemExpandedHandler
    {
        public int CallCount { get; private set; }

        public int Invoke()
        {
            CallCount++;
            return 0;
        }
    }

    private sealed class CollapsedHandler : IAvnTreeViewItemCollapsedHandler
    {
        public int CallCount { get; private set; }

        public int Invoke()
        {
            CallCount++;
            return 0;
        }
    }

    [Fact]
    public void Tool_tip_tip_crosses_as_a_string_and_a_control_tip_reads_back_as_null()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);
        Assert.Equal(0, factory.GetToolTipStatics(out var projectedStatics));
        Assert.NotNull(projectedStatics);
        var button = Target<Button>(projected);

        Through<IAvnToolTipStatics>(projectedStatics, statics =>
        {
            Through<IAvnControl>(projected, target =>
            {
                Assert.Equal(0, statics.GetTip(target, out var initial));
                Assert.Null(initial);

                Assert.Equal(0, statics.SetTip(target, "Save the document"));
                Assert.Equal(0, statics.GetTip(target, out var tip));
                Assert.Equal("Save the document", tip);

                Assert.Equal(0, statics.SetShowDelay(target, 250));
                Assert.Equal(0, statics.GetShowDelay(target, out var showDelay));
                Assert.Equal(250, showDelay);

                Assert.Equal(0, statics.SetPlacement(target, (int)PlacementMode.Top));
                Assert.Equal(0, statics.GetPlacement(target, out var placement));
                Assert.Equal((int)PlacementMode.Top, placement);

                Assert.Equal(0, statics.SetServiceEnabled(target, 0));
                Assert.Equal(0, statics.GetServiceEnabled(target, out var serviceEnabled));
                Assert.Equal(0, serviceEnabled);

                // An empty string clears the tip rather than storing an empty tooltip.
                Assert.Equal(0, statics.SetTip(target, string.Empty));
                Assert.Null(ToolTip.GetTip(button));

                Assert.Equal(0, statics.SetTip(target, "Back again"));
            });
        });

        Assert.Equal("Back again", ToolTip.GetTip(button));
        Assert.Equal(250, ToolTip.GetShowDelay(button));
        Assert.Equal(PlacementMode.Top, ToolTip.GetPlacement(button));
        Assert.False(ToolTip.GetServiceEnabled(button));

        // A control-valued tip is legal managed-side and reads back as null, because wave A
        // carries text only.
        ToolTip.SetTip(button, new TextBlock { Text = "rich" });
        Through<IAvnToolTipStatics>(projectedStatics, statics =>
            Through<IAvnControl>(projected, target =>
            {
                Assert.Equal(0, statics.GetTip(target, out var tip));
                Assert.Null(tip);
            }));
    }

    [Fact]
    public void Tool_tip_statics_reject_a_null_target()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.GetToolTipStatics(out var statics));
        Assert.NotNull(statics);
        Assert.True(statics.SetTip(null, "orphan") < 0);
        Assert.True(statics.GetTip(null, out _) < 0);
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
