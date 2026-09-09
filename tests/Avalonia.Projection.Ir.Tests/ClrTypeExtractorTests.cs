using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class ClrTypeExtractorTests
{
    private static readonly Type[] KernelTypes =
    [
        typeof(AvaloniaObject),
        typeof(StyledElement),
        typeof(Control),
        typeof(ContentControl),
        typeof(HeaderedContentControl),
        typeof(ItemsControl),
        typeof(HeaderedItemsControl),
        typeof(SelectingItemsControl),
        typeof(Decorator),
        typeof(Border),
        typeof(Panel),
        typeof(Grid),
        typeof(Canvas),
        typeof(DockPanel),
        typeof(Window),
        typeof(StackPanel),
        typeof(TextBlock),
        typeof(Image),
        typeof(TemplatedControl),
        typeof(Button),
        typeof(ToggleButton),
        typeof(CheckBox),
        typeof(RadioButton),
        typeof(ToggleSwitch),
        typeof(Expander),
        typeof(ListBox),
        typeof(ComboBox),
        typeof(ListBoxItem),
        typeof(ComboBoxItem),
        typeof(TabControl),
        typeof(TabItem),
        typeof(TreeView),
        typeof(TreeViewItem),
        typeof(ToolTip),
        typeof(FlyoutBase),
        typeof(PopupFlyoutBase),
        typeof(Flyout),
        typeof(MenuBase),
        typeof(Menu),
        typeof(HeaderedSelectingItemsControl),
        typeof(MenuItem),
        typeof(SplitView),
        typeof(DatePicker),
        typeof(TimePicker),
        typeof(WrapPanel),
        typeof(UniformGrid),
        typeof(RelativePanel),
        typeof(Viewbox),
        typeof(FlexPanel),
        typeof(Thumb),
        typeof(GridSplitter),
        typeof(RepeatButton),
        typeof(DropDownButton),
        typeof(SplitButton),
        typeof(ToggleSplitButton),
        typeof(HyperlinkButton),
        typeof(ContextMenu),
        typeof(MenuFlyout),
        typeof(Spinner),
        typeof(ButtonSpinner),
        typeof(NumericUpDown),
        typeof(AutoCompleteBox),
        typeof(MaskedTextBox),
        typeof(SelectableTextBlock),
        typeof(Calendar),
        typeof(CalendarDatePicker),
        typeof(Carousel),
        typeof(TransitioningContentControl),
        typeof(Label),
        typeof(Separator),
        typeof(GroupBox),
        typeof(UserControl),
        typeof(LayoutTransformControl),
        typeof(Avalonia.Controls.Shapes.Shape),
        typeof(Avalonia.Controls.Shapes.Rectangle),
        typeof(Avalonia.Controls.Shapes.Ellipse),
        typeof(Avalonia.Controls.Shapes.Line),
        typeof(Avalonia.Controls.Shapes.Path),
        typeof(Avalonia.Controls.Shapes.Polygon),
        typeof(Avalonia.Controls.Shapes.Polyline),
        typeof(Avalonia.Controls.Shapes.Arc),
        typeof(Avalonia.Controls.Shapes.Sector),
        typeof(Popup),
        typeof(TrayIcon),
        typeof(Avalonia.Controls.Notifications.WindowNotificationManager),
        typeof(Avalonia.Controls.Notifications.NotificationCard),
        typeof(RefreshContainer),
        typeof(CommandBar),
        typeof(CommandBarButton),
        typeof(CommandBarToggleButton),
        typeof(CommandBarSeparator),
        typeof(PipsPager),
        typeof(ThemeVariantScope),
        typeof(IconElement),
        typeof(PathIcon),
        typeof(TableView),
        typeof(TableViewColumn),
        typeof(TableViewRow),
        typeof(TableViewCell),
        typeof(TextBox),
        typeof(ScrollViewer),
        typeof(RangeBase),
        typeof(Slider),
        typeof(ProgressBar),
    ];

    [Fact]
    public void Projects_kernel_types_properties_commands_and_nearest_bases()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal(ProjectionIr.CurrentVersion, ir.Version);
        Assert.Equal(KernelTypes.Length, ir.Types.Count);

        var contentControl = Type(ir, "IAvnContentControl");
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", contentControl.BaseFullName);
        var content = contentControl.Properties.Single(p => p.Name == nameof(ContentControl.Content));
        Assert.Equal(MarshallingKind.ComInterface, content.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", content.InterfaceName);
        Assert.True(content.IsNullable);

        var window = Type(ir, "IAvnWindow");
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", window.BaseFullName);
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.Title));
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.ExtendClientAreaToDecorationsHint));
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.IsExtendedIntoWindowDecorations));
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.WindowDecorationMargin));
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.OffScreenMargin));
        Assert.Contains(window.Properties, p => p.Name == nameof(Window.IsDialog));
        Assert.Contains(window.Events, e => e.Name == nameof(Window.Closing));
        Assert.Contains(window.Methods, m => m.Name == nameof(Window.Show) && m.Parameters.Count == 0);
        Assert.Contains(window.Methods, m => m.Name == nameof(Window.Close) && m.Parameters.Count == 0);

        var stackPanel = Type(ir, "IAvnStackPanel");
        Assert.Equal("Avalonia.Host.Com.IAvnPanel", stackPanel.BaseFullName);
        Assert.Equal(MarshallingKind.I32, stackPanel.Properties.Single(p => p.Name == nameof(StackPanel.Orientation)).Kind);
        Assert.Equal(MarshallingKind.F64, stackPanel.Properties.Single(p => p.Name == nameof(StackPanel.Spacing)).Kind);
        var children = Type(ir, "IAvnPanel").Properties.Single(p => p.Name == nameof(Panel.Children));
        Assert.Equal(MarshallingKind.ComCollection, children.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControlList", children.InterfaceName);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", children.ElementInterfaceName);
        Assert.Equal(MarshallingKind.ComInterface, children.ElementKind);

        var text = Type(ir, "IAvnTextBlock").Properties.Single(p => p.Name == nameof(TextBlock.Text));
        Assert.Equal(MarshallingKind.StringUtf16, text.Kind);
        Assert.True(text.IsNullable);

        var click = Assert.Single(Type(ir, "IAvnButton").Events);
        Assert.Equal(nameof(Button.Click), click.Name);
        Assert.Equal("Avalonia.Host.Com.IAvnButtonClickHandler", click.HandlerInterfaceName);
        Assert.Equal(EventPayloadKind.None, click.PayloadKind);
        Assert.False(string.IsNullOrWhiteSpace(click.HandlerInterfaceIid));

        var isChecked = Type(ir, "IAvnToggleButton").Properties
            .Single(property => property.Name == nameof(ToggleButton.IsChecked));
        Assert.Equal(MarshallingKind.NullableBool, isChecked.Kind);

        var controlEvents = Type(ir, "IAvnControl").Events;
        Assert.Equal(6, controlEvents.Count);
        var sizeChanged = controlEvents.Single(@event => @event.Name == "SizeChanged");
        Assert.Equal(EventPayloadKind.Fields, sizeChanged.PayloadKind);
        Assert.Equal(2, sizeChanged.Parameters.Count);
        var keyDown = controlEvents.Single(@event => @event.Name == "KeyDown");
        Assert.Equal(EventPayloadKind.Fields, keyDown.PayloadKind);
        Assert.Equal(5, keyDown.Parameters.Count);
        Assert.Equal(
            ParameterDirection.InOut,
            keyDown.Parameters.Single(parameter => parameter.Name == "Handled").Direction);
        Assert.All(
            controlEvents.Where(@event => @event.Name.StartsWith("Pointer", StringComparison.Ordinal)),
            @event => Assert.Equal(EventPayloadKind.None, @event.PayloadKind));
        Assert.All(
            Type(ir, "IAvnControl").Properties.Where(property =>
                property.Name is nameof(Control.Width) or nameof(Control.Height)),
            property => Assert.Equal(MarshallingKind.F64, property.Kind));

        var toggleSwitch = Type(ir, "IAvnToggleSwitch");
        Assert.Equal("Avalonia.Host.Com.IAvnToggleButton", toggleSwitch.BaseFullName);
        // The On/OffContent properties cross as controls; their templates cross as
        // the data-template interface.
        Assert.All(
            toggleSwitch.Properties.Where(property =>
                property.Name is nameof(ToggleSwitch.OnContent) or nameof(ToggleSwitch.OffContent)),
            property => Assert.Equal("Avalonia.Host.Com.IAvnControl", property.InterfaceName));
        Assert.All(
            toggleSwitch.Properties.Where(property =>
                property.Name is nameof(ToggleSwitch.OnContentTemplate) or nameof(ToggleSwitch.OffContentTemplate)),
            property => Assert.Equal("Avalonia.Host.Com.IAvnDataTemplate", property.InterfaceName));

        var expander = Type(ir, "IAvnExpander");
        Assert.Equal("Avalonia.Host.Com.IAvnHeaderedContentControl", expander.BaseFullName);
        // U25 added the vetoable Expanding/Collapsing pair.
        Assert.Equal(4, expander.Events.Count);
        Assert.Contains(expander.Events, @event =>
            @event.Name == nameof(Expander.Expanding) &&
            @event.Parameters.Single().Name == "Cancel");

        var items = Type(ir, "IAvnItemsControl").Properties.Single(p => p.Name == "Items");
        Assert.Equal("Avalonia.Host.Com.IAvnItemList", items.InterfaceName);
        Assert.Equal(MarshallingKind.ComInterface, items.ElementKind);

        var styledElement = Type(ir, "IAvnStyledElement");
        var classes = styledElement.Properties.Single(property => property.Name == "Classes");
        Assert.Equal(MarshallingKind.StringUtf16, classes.ElementKind);
        var name = styledElement.Properties.Single(property => property.Name == nameof(StyledElement.Name));
        Assert.Equal(MarshallingKind.StringUtf16, name.Kind);
        Assert.True(name.IsNullable);
        Assert.Contains(ir.AttachedProperties, property =>
            property.OwnerName == nameof(Grid) && property.Name == "Row");
    }

    [Fact]
    public void Projects_layout_members_onto_control_decorator_and_window()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var control = Type(ir, "IAvnControl");
        Assert.Equal(
            MarshallingKind.Thickness,
            control.Properties.Single(property => property.Name == nameof(Control.Margin)).Kind);
        Assert.All(
            control.Properties.Where(property => property.Name is
                nameof(Control.MinWidth) or nameof(Control.MinHeight) or
                nameof(Control.MaxWidth) or nameof(Control.MaxHeight) or nameof(Control.Opacity)),
            property => Assert.Equal(MarshallingKind.F64, property.Kind));
        Assert.Equal(
            MarshallingKind.Bool,
            control.Properties.Single(property => property.Name == nameof(Control.IsVisible)).Kind);
        Assert.All(
            control.Properties.Where(property => property.Name is
                nameof(Control.HorizontalAlignment) or nameof(Control.VerticalAlignment)),
            property =>
            {
                Assert.Equal(MarshallingKind.I32, property.Kind);
                Assert.True(property.CanRead && property.CanWrite);
            });
        Assert.All(
            new[] { "HorizontalAlignment", "VerticalAlignment", "WindowState" },
            enumName => Assert.Contains(ir.Enums, projected => projected.Name == enumName));

        Assert.Equal(
            MarshallingKind.Thickness,
            Type(ir, "IAvnDecorator").Properties
                .Single(property => property.Name == nameof(Decorator.Padding)).Kind);

        var window = Type(ir, "IAvnWindow");
        Assert.Equal(
            MarshallingKind.Bool,
            window.Properties.Single(property => property.Name == nameof(Window.CanResize)).Kind);
        Assert.Equal(
            MarshallingKind.I32,
            window.Properties.Single(property => property.Name == nameof(Window.WindowState)).Kind);
    }

    [Fact]
    public void Layout_members_keep_the_abi_version_of_the_interfaces_they_widened()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // AvaloniaObject projects no members, so its flattened vtable is unchanged and it
        // must keep the IID it published at version 2.
        var avaloniaObject = Type(ir, "IAvnAvaloniaObject");
        Assert.Equal(2, avaloniaObject.AbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid(avaloniaObject.FullName, 2),
            avaloniaObject.Iid);

        // StyledElement is unchanged. Control grew ContextMenu/IsLoaded (wave), Decorator
        // republishes with every Control growth plus its own; U19 grew ContentControl
        // above Decorator too.
        Assert.Equal(6, Type(ir, "IAvnStyledElement").AbiVersion);
        Assert.Equal(8, Type(ir, "IAvnControl").AbiVersion);
        Assert.Equal(10, Type(ir, "IAvnDecorator").AbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid(
                Type(ir, "IAvnDecorator").FullName, 10),
            Type(ir, "IAvnDecorator").Iid);
    }

    [Fact]
    public void Chrome_members_keep_the_abi_version_of_the_interfaces_they_widened()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // Border and Panel grew slots in the chrome wave and nothing since has moved them,
        // so they stay on their version 4 IIDs. TemplatedControl and TextBlock moved in wave M.
        Assert.All(
            new[] { "IAvnPanel", "IAvnCanvas", "IAvnDockPanel" },
            name => { var type = Type(ir, name); Assert.Equal(11, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 11),
                    type.Iid);
            });
        Assert.All(
            new[] { "IAvnBorder", "IAvnStackPanel" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(12, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 12),
                    type.Iid);
            });

        // The factory grew a creator per wave A control plus GetToolTipStatics, then one per
        // constructible wave B type, then one per constructible wave C type, so it has moved
        // three times off the version 2 IID it published for CreateSolidColorBrush.
        Assert.Equal(13, ir.FactoryAbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid("Avalonia.Host.Com.IAvnControlFactory", 13),
            ir.FactoryIid);
    }

    [Fact]
    public void Completeness_members_keep_the_abi_version_of_every_widened_interface()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // Grid is a Panel descendant, so it republishes with every Control/StyledElement
        // growth in addition to its own waves.
        Assert.Equal(12, Type(ir, "IAvnGrid").AbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid(Type(ir, "IAvnGrid").FullName, 12),
            Type(ir, "IAvnGrid").Iid);

        // Completeness-wave types sat at 5 until wave M grew TemplatedControl under them,
        // and every StyledElement/Control growth since republishes them again. The two
        // items-derived ones republish with the items lineage too (U17).
        Assert.All(
            new[] { "IAvnContentControl", "IAvnHeaderedContentControl", "IAvnExpander" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(13, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 13),
                    type.Iid);
            });
        Assert.All(
            new[] { "IAvnListBoxItem", "IAvnComboBoxItem" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(15, type.AbiVersion);
                Assert.Equal(
                        ClrTypeExtractor.CreateDeterministicIid(type.FullName, 15),
                        type.Iid);
            });

        var window = Type(ir, "IAvnWindow");
        Assert.Equal(18, window.AbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid(window.FullName, 18),
            window.Iid);
    }

    [Fact]
    public void Wave_a_controls_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // Every wave A interface is brand new, so it publishes at version 1 rather than
        // inheriting the version its neighbours happen to sit on.
        Assert.Equal(8, Type(ir, "IAvnImage").AbiVersion);
        // U17 republished TabItem with the items lineage; ToolTip was untouched.
        Assert.Equal(11, Type(ir, "IAvnTabItem").AbiVersion);
        Assert.Equal(9, Type(ir, "IAvnToolTip").AbiVersion);
        Assert.All(
            new[]
            {
                "IAvnHeaderedItemsControl", "IAvnTabControl",
                "IAvnTreeView", "IAvnTreeViewItem",
            },
            name => { var type = Type(ir, name); Assert.Equal(12, type.AbiVersion);
            });

        // Wave M grew TemplatedControl, so every previously published interface below it
        // moved; Image sits on Control and is unchanged.
        var pinned = new Dictionary<string, int>(StringComparer.Ordinal)
        {
            ["IAvnAvaloniaObject"] = 2,
            ["IAvnStyledElement"] = 6,
            ["IAvnControl"] = 8,
            ["IAvnDecorator"] = 10,
            ["IAvnItemsControl"] = 15,
            ["IAvnSelectingItemsControl"] = 15,
            ["IAvnContentControl"] = 13,
            ["IAvnHeaderedContentControl"] = 13,
        };
        Assert.All(pinned, entry =>
        {
            var type = Type(ir, entry.Key);
            Assert.Equal(entry.Value, type.AbiVersion);
            Assert.Equal(
                ClrTypeExtractor.CreateDeterministicIid(type.FullName, entry.Value),
                type.Iid);
        });
    }

    [Fact]
    public void Projects_the_wave_a_controls_onto_the_types_that_declare_them()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // Image.Source is an IImage, which has no ABI shape, so it crosses as the source string
        // the host resolves into a bitmap. The converter, not the CLR type, owns the conversion.
        var image = Type(ir, "IAvnImage");
        Assert.Equal("Avalonia.Host.Com.IAvnControl", image.BaseFullName);
        var source = image.Properties.Single(property => property.Name == nameof(Image.Source));
        Assert.Equal(MarshallingKind.StringUtf16, source.Kind);
        Assert.Equal("Avalonia.Media.IImage", source.ManagedTypeName);
        Assert.Equal("Avalonia.Host.Com.AvnImageSource", source.StringConverterTypeName);
        Assert.True(source.IsNullable);
        Assert.True(source.CanRead && source.CanWrite);
        Assert.All(
            new[] { nameof(Image.Stretch), nameof(Image.StretchDirection), nameof(Image.BlendMode) },
            name => Assert.Equal(
                MarshallingKind.I32,
                image.Properties.Single(property => property.Name == name).Kind));

        // TabControl derives from SelectingItemsControl, so Items and SelectedIndex are
        // inherited rather than redeclared. U18 added SelectedContent as a Variant.
        var tabControl = Type(ir, "IAvnTabControl");
        Assert.Equal("Avalonia.Host.Com.IAvnSelectingItemsControl", tabControl.BaseFullName);
        Assert.Equal(
            [nameof(TabControl.ContentTemplate), nameof(TabControl.HorizontalContentAlignment),
                nameof(TabControl.IndicatorTemplate), nameof(TabControl.SelectedContent),
                nameof(TabControl.SelectedContentTemplate), nameof(TabControl.TabStripPlacement),
                nameof(TabControl.VerticalContentAlignment)],
            tabControl.Properties.Select(property => property.Name).OrderBy(n => n, StringComparer.Ordinal));
        Assert.Equal(
            MarshallingKind.Variant,
            tabControl.Properties.Single(property => property.Name == nameof(TabControl.SelectedContent)).Kind);
        Assert.Equal(
            MarshallingKind.DataTemplate,
            tabControl.Properties.Single(property => property.Name == nameof(TabControl.ContentTemplate)).Kind);

        var tabItem = Type(ir, "IAvnTabItem");
        Assert.Equal("Avalonia.Host.Com.IAvnHeaderedContentControl", tabItem.BaseFullName);
        Assert.Equal(
            MarshallingKind.Bool,
            tabItem.Properties.Single(property => property.Name == nameof(TabItem.IsSelected)).Kind);
        // U27: TabStripPlacement is a Dock? that crosses as a nullable string through the
        // AvnDock host converter (null = unset); it is read-only on the ABI because the CLR
        // setter is internal.
        var placement = tabItem.Properties
            .Single(property => property.Name == nameof(TabItem.TabStripPlacement));
        Assert.Equal(MarshallingKind.StringUtf16, placement.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnDock", placement.StringConverterTypeName);
        Assert.True(placement.IsNullable);
        Assert.True(placement.CanRead);
        Assert.False(placement.CanWrite);

        // TreeView is an ItemsControl, not a SelectingItemsControl, so it carries Items but no
        // SelectedIndex. U18 projects SelectedItem as a Variant.
        var treeView = Type(ir, "IAvnTreeView");
        Assert.Equal("Avalonia.Host.Com.IAvnItemsControl", treeView.BaseFullName);
        Assert.DoesNotContain(treeView.Properties, property => property.Name == "SelectedIndex");
        Assert.Equal(
            MarshallingKind.Variant,
            treeView.Properties.Single(property => property.Name == nameof(TreeView.SelectedItem)).Kind);
        Assert.Contains(
            treeView.Methods,
            method => method.Name == "TreeContainerFromItemWithObject");
        var expand = treeView.Methods.Single(method => method.ManagedName == nameof(TreeView.ExpandSubTree));
        Assert.Equal("Avalonia.Host.Com.IAvnTreeViewItem", expand.Parameters.Single().InterfaceName);
        Assert.Contains(treeView.Methods, method => method.Name == nameof(TreeView.UnselectAll));
        // TreeView declares its own SelectionChanged because it is not a SelectingItemsControl.
        var treeSelectionChanged = treeView.Events.Single();
        Assert.Equal("Avalonia.Host.Com.IAvnTreeViewSelectionChangedHandler", treeSelectionChanged.HandlerInterfaceName);
        Assert.Equal(EventPayloadKind.None, treeSelectionChanged.PayloadKind);

        // TreeViewItem's Header comes from HeaderedItemsControl and crosses as a control, the
        // same shape HeaderedContentControl.Header already uses.
        var treeViewItem = Type(ir, "IAvnTreeViewItem");
        Assert.Equal("Avalonia.Host.Com.IAvnHeaderedItemsControl", treeViewItem.BaseFullName);
        var header = Type(ir, "IAvnHeaderedItemsControl").Properties
            .Single(p => p.Name == nameof(HeaderedItemsControl.Header));
        Assert.Equal(MarshallingKind.ComInterface, header.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", header.InterfaceName);
        Assert.All(
            new[] { nameof(TreeViewItem.IsExpanded), nameof(TreeViewItem.IsSelected) },
            name => Assert.Equal(
                MarshallingKind.Bool,
                treeViewItem.Properties.Single(property => property.Name == name).Kind));
        var level = treeViewItem.Properties.Single(property => property.Name == nameof(TreeViewItem.Level));
        Assert.True(level.CanRead);
        Assert.False(level.CanWrite);
        Assert.Equal(
            ["Collapsed", "Expanded"],
            treeViewItem.Events.Select(@event => @event.Name).OrderBy(n => n, StringComparer.Ordinal));
        Assert.All(treeViewItem.Events, @event => Assert.Equal(EventPayloadKind.None, @event.PayloadKind));
    }

    [Fact]
    public void Wave_b_controls_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal(1, Type(ir, "IAvnFlyoutBase").AbiVersion);
        // PopupFlyoutBase stayed; Flyout grew ContentTemplate in U19.
        Assert.Equal(6, Type(ir, "IAvnPopupFlyoutBase").AbiVersion);
        Assert.Equal(4, Type(ir, "IAvnFlyout").AbiVersion);
        Assert.All(
            new[] { "IAvnSplitView" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(9, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 9),
                    type.Iid);
            });
        Assert.All(
            new[] { "IAvnDatePicker", "IAvnTimePicker" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(11, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 11),
                    type.Iid);
            });
        Assert.All(
            new[] { "IAvnMenuBase", "IAvnMenu", "IAvnHeaderedSelectingItemsControl" }, name => Assert.Equal(12, Type(ir, name).AbiVersion));
        Assert.Equal(15, Type(ir, "IAvnMenuItem").AbiVersion);

        var pinned = new Dictionary<string, int>(StringComparer.Ordinal)
        {
            ["IAvnAvaloniaObject"] = 2,
            ["IAvnControl"] = 8,
            ["IAvnItemsControl"] = 15,
            ["IAvnSelectingItemsControl"] = 15,
            ["IAvnTemplatedControl"] = 12,
            ["IAvnContentControl"] = 13,
        };
        Assert.All(pinned, entry =>
        {
            var type = Type(ir, entry.Key);
            Assert.Equal(entry.Value, type.AbiVersion);
            Assert.Equal(
                ClrTypeExtractor.CreateDeterministicIid(type.FullName, entry.Value),
                type.Iid);
        });

        // Reusing a retired IID would silently hand a stale consumer a changed contract, so no
        // two projected interfaces may ever share one.
        Assert.Equal(
            ir.Types.Count,
            ir.Types.Select(type => type.Iid).Distinct(StringComparer.Ordinal).Count());
    }

    [Fact]
    public void Projects_the_flyout_trio_with_an_imperative_show_rather_than_an_attached_property()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // FlyoutBase derives from AvaloniaObject, not Control, so the projected interface hangs
        // straight off IAvnAvaloniaObject and inserts no slot into anything that shipped.
        var flyoutBase = Type(ir, "IAvnFlyoutBase");
        Assert.Equal("Avalonia.Host.Com.IAvnAvaloniaObject", flyoutBase.BaseFullName);
        Assert.False(flyoutBase.IsConstructible);

        // ShowAt is how a flyout reaches a control: the attached-property pipeline carries
        // scalars and strings only, so there is no COM-valued AttachedFlyout in this wave.
        var showAt = flyoutBase.Methods.Single(method => method.ManagedName == "ShowAt");
        var placementTarget = showAt.Parameters.Single();
        Assert.Equal(MarshallingKind.ComInterface, placementTarget.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", placementTarget.InterfaceName);
        Assert.Contains(flyoutBase.Methods, method => method.Name == "Hide");
        Assert.DoesNotContain(
            ir.AttachedProperties,
            property => property.Name is "AttachedFlyout" or "Flyout");
        Assert.Equal(
            MarshallingKind.Command,
            Type(ir, "IAvnButton").Properties.Single(p => p.Name == nameof(Button.Command)).Kind);

        var target = flyoutBase.Properties.Single(property => property.Name == "Target");
        Assert.Equal(MarshallingKind.ComInterface, target.Kind);
        Assert.True(target.CanRead);
        Assert.False(target.CanWrite);

        // PopupFlyoutBase re-declares ShowAt/Hide as sealed overrides; the flattened vtable
        // publishes each exactly once, from the base that declares it.
        var popupFlyoutBase = Type(ir, "IAvnPopupFlyoutBase");
        Assert.Equal("Avalonia.Host.Com.IAvnFlyoutBase", popupFlyoutBase.BaseFullName);
        // U34 projects the pointer-tracking ShowAt overload; the sealed ShowAt/Hide
        // overrides of the base slots stay suppressed.
        var showAtPointer = popupFlyoutBase.Methods.Single(m => m.Name == "ShowAtWithControlAndBoolean");
        Assert.Contains(showAtPointer.Parameters, p => p.Name == "showAtPointer");
        Assert.All(
            new[] { "Placement", "ShowMode" },
            name => Assert.Equal(
                MarshallingKind.I32,
                popupFlyoutBase.Properties.Single(property => property.Name == name).Kind));

        Assert.All(
            new[] { "PlacementAnchor", "PlacementGravity", "PlacementConstraintAdjustment" },
            name =>
            {
                var property = popupFlyoutBase.Properties.Single(p => p.Name == name);
                Assert.Equal(MarshallingKind.I32, property.Kind);
            });
        Assert.Contains(ir.Enums, e => e.Name == "PopupAnchor" && e.IsFlags);

        // Closing is the one wave B event with a payload, and Cancel is written back.
        var closing = popupFlyoutBase.Events.Single(@event => @event.Name == "Closing");
        Assert.Equal(EventPayloadKind.Fields, closing.PayloadKind);
        var cancel = closing.Parameters.Single();
        Assert.Equal("Cancel", cancel.Name);
        Assert.Equal(MarshallingKind.Bool, cancel.Kind);
        Assert.Equal(ParameterDirection.InOut, cancel.Direction);

        var flyout = Type(ir, "IAvnFlyout");
        Assert.Equal("Avalonia.Host.Com.IAvnPopupFlyoutBase", flyout.BaseFullName);
        Assert.True(flyout.IsConstructible);
        var content = flyout.Properties.Single(property => property.Name == "Content");
        Assert.Equal(MarshallingKind.ComInterface, content.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", content.InterfaceName);
        Assert.Equal("System.Object", content.ManagedTypeName);
    }

    [Fact]
    public void Projects_the_imperative_menu_pair_without_an_icommand()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // MenuBase owns the open state and the commands; Menu inherits all of it and declares
        // nothing, so Open/Close occupy one slot each rather than two.
        var menuBase = Type(ir, "IAvnMenuBase");
        Assert.Equal("Avalonia.Host.Com.IAvnSelectingItemsControl", menuBase.BaseFullName);
        Assert.Equal(
            ["Close", "Open"],
            menuBase.Methods.Select(method => method.Name).OrderBy(n => n, StringComparer.Ordinal));
        var isOpen = menuBase.Properties.Single();
        Assert.Equal("IsOpen", isOpen.Name);
        // The managed setter is protected, so the ABI publishes a getter only rather than
        // inventing a writable open state the control does not have.
        Assert.True(isOpen.CanRead);
        Assert.False(isOpen.CanWrite);

        var menu = Type(ir, "IAvnMenu");
        Assert.Equal("Avalonia.Host.Com.IAvnMenuBase", menu.BaseFullName);
        Assert.True(menu.IsConstructible);
        Assert.Empty(menu.Properties);
        Assert.Empty(menu.Methods);

        // MenuItem's Header comes from the newly projected HeaderedSelectingItemsControl, and
        // Items is inherited all the way from ItemsControl.
        var headered = Type(ir, "IAvnHeaderedSelectingItemsControl");
        Assert.Equal("Avalonia.Host.Com.IAvnSelectingItemsControl", headered.BaseFullName);
        var header = headered.Properties.Single(p => p.Name == nameof(HeaderedSelectingItemsControl.Header));
        Assert.Equal(MarshallingKind.ComInterface, header.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", header.InterfaceName);
        var headerTemplate = headered.Properties.Single(p => p.Name == nameof(HeaderedSelectingItemsControl.HeaderTemplate));
        Assert.Equal(MarshallingKind.DataTemplate, headerTemplate.Kind);

        var menuItem = Type(ir, "IAvnMenuItem");
        Assert.Equal("Avalonia.Host.Com.IAvnHeaderedSelectingItemsControl", menuItem.BaseFullName);
        var icon = menuItem.Properties.Single(property => property.Name == nameof(MenuItem.Icon));
        Assert.Equal(MarshallingKind.ComInterface, icon.Kind);
        Assert.Equal(
            MarshallingKind.Bool,
            menuItem.Properties.Single(property => property.Name == nameof(MenuItem.IsChecked)).Kind);
        Assert.Equal(
            MarshallingKind.I32,
            menuItem.Properties.Single(property => property.Name == nameof(MenuItem.ToggleType)).Kind);

        // Click is the imperative pair of the command surface: the command itself and
        // its scalar parameter cross; KeyGestures stay in the gap report.
        Assert.Contains(menuItem.Events, @event =>
            @event.Name == nameof(MenuItem.Click) && @event.PayloadKind == EventPayloadKind.None);
        Assert.Equal(
            MarshallingKind.Command,
            menuItem.Properties.Single(property => property.Name == nameof(MenuItem.Command)).Kind);
        Assert.Equal(
            MarshallingKind.Variant,
            menuItem.Properties.Single(property => property.Name == nameof(MenuItem.CommandParameter)).Kind);
        Assert.True(
            menuItem.Properties
                .Single(property => property.Name == nameof(MenuItem.CommandParameter))
                .IsNullable);
        // U27: HotKey and InputGesture are KeyGestures that cross through the type's own
        // Parse/ToString round-trip as ordinary UTF-16 strings.
        Assert.All(
            new[] { nameof(MenuItem.HotKey), nameof(MenuItem.InputGesture) },
            name =>
            {
                var gesture = menuItem.Properties.Single(property => property.Name == name);
                Assert.Equal(MarshallingKind.StringUtf16, gesture.Kind);
                Assert.True(gesture.IsNullable);
                Assert.True(gesture.CanRead && gesture.CanWrite);
            });
    }

    [Fact]
    public void Projects_split_view_panes_as_controls_and_brushes()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var splitView = Type(ir, "IAvnSplitView");
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", splitView.BaseFullName);

        var pane = splitView.Properties.Single(property => property.Name == nameof(SplitView.Pane));
        Assert.Equal(MarshallingKind.ComInterface, pane.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", pane.InterfaceName);
        Assert.True(pane.IsNullable);

        var background = splitView.Properties
            .Single(property => property.Name == nameof(SplitView.PaneBackground));
        Assert.Equal(MarshallingKind.Brush, background.Kind);

        Assert.All(
            new[] { nameof(SplitView.DisplayMode), nameof(SplitView.PanePlacement) },
            name => Assert.Equal(
                MarshallingKind.I32,
                splitView.Properties.Single(property => property.Name == name).Kind));
        Assert.All(
            new[] { nameof(SplitView.OpenPaneLength), nameof(SplitView.CompactPaneLength) },
            name => Assert.Equal(
                MarshallingKind.F64,
                splitView.Properties.Single(property => property.Name == name).Kind));
        Assert.Equal(
            ["PaneClosed", "PaneClosing", "PaneOpened", "PaneOpening"],
            splitView.Events.Select(@event => @event.Name).OrderBy(n => n, StringComparer.Ordinal));
        var paneOpening = splitView.Events.Single(@event => @event.Name == "PaneOpening");
        Assert.Equal(EventPayloadKind.Fields, paneOpening.PayloadKind);
        Assert.Single(paneOpening.Parameters, parameter => parameter.Name == "Cancel");
        Assert.Equal(
            ParameterDirection.InOut,
            paneOpening.Parameters.Single().Direction);
    }

    [Fact]
    public void Projects_picker_dates_and_times_as_converted_strings()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // DateTimeOffset and TimeSpan have no ABI shape here, so they ride the same host-side
        // converter mechanism Image.Source uses rather than a minted date struct.
        var datePicker = Type(ir, "IAvnDatePicker");
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", datePicker.BaseFullName);

        var selectedDate = datePicker.Properties
            .Single(property => property.Name == nameof(DatePicker.SelectedDate));
        Assert.Equal(MarshallingKind.StringUtf16, selectedDate.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnDateTimeOffset", selectedDate.StringConverterTypeName);
        Assert.True(selectedDate.IsNullable);

        // MinYear/MaxYear have no absent state, so they take the non-nullable converter.
        Assert.All(
            new[] { nameof(DatePicker.MinYear), nameof(DatePicker.MaxYear) },
            name =>
            {
                var property = datePicker.Properties.Single(p => p.Name == name);
                Assert.Equal(MarshallingKind.StringUtf16, property.Kind);
                Assert.Equal(
                    "Avalonia.Host.Com.AvnDateTimeOffsetValue",
                    property.StringConverterTypeName);
                Assert.False(property.IsNullable);
            });
        Assert.Contains(datePicker.Methods, method => method.Name == nameof(DatePicker.Clear));

        // U28: SelectedDateChanged crosses with the old/new pair as nullable DateTime fields.
        var dateChanged = Assert.Single(datePicker.Events);
        Assert.Equal(nameof(DatePicker.SelectedDateChanged), dateChanged.Name);
        Assert.Equal(EventPayloadKind.Fields, dateChanged.PayloadKind);
        Assert.Equal(
            new[] { "OldDate", "NewDate" },
            dateChanged.Parameters.Select(p => p.Name));
        Assert.All(dateChanged.Parameters, p => Assert.Equal(MarshallingKind.DateTimeI64, p.Kind));
        Assert.All(dateChanged.Parameters, p => Assert.True(p.IsNullable));

        var timePicker = Type(ir, "IAvnTimePicker");
        var selectedTime = timePicker.Properties
            .Single(property => property.Name == nameof(TimePicker.SelectedTime));
        Assert.Equal(MarshallingKind.StringUtf16, selectedTime.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnTimeSpan", selectedTime.StringConverterTypeName);
        Assert.True(selectedTime.IsNullable);
        Assert.Equal(
            MarshallingKind.I32,
            timePicker.Properties.Single(p => p.Name == nameof(TimePicker.MinuteIncrement)).Kind);
        // U28: SelectedTimeChanged crosses with the old/new pair as nullable TimeSpan fields.
        var timeChanged = Assert.Single(timePicker.Events);
        Assert.Equal(nameof(TimePicker.SelectedTimeChanged), timeChanged.Name);
        Assert.Equal(EventPayloadKind.Fields, timeChanged.PayloadKind);
        Assert.Equal(
            new[] { "OldTime", "NewTime" },
            timeChanged.Parameters.Select(p => p.Name));
        Assert.All(timeChanged.Parameters, p => Assert.Equal(MarshallingKind.TimeSpanI64, p.Kind));
        Assert.All(timeChanged.Parameters, p => Assert.True(p.IsNullable));
    }

    [Fact]
    public void Projects_tool_tip_attached_properties_with_a_string_tip()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var tip = ir.AttachedProperties.Single(property =>
            property.OwnerName == nameof(ToolTip) && property.Name == "Tip");
        Assert.Equal("Avalonia.Host.Com.IAvnToolTipStatics", tip.StaticsInterfaceName);
        Assert.Equal(1, tip.StaticsInterfaceAbiVersion);
        // The managed property is an object; the ABI carries text and only text.
        Assert.Equal("System.Object", tip.ManagedTypeName);
        Assert.Equal(MarshallingKind.StringUtf16, tip.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnToolTipTip", tip.StringConverterTypeName);
        Assert.True(tip.IsNullable);

        // The scalar tooltip members come through unchanged, and none of them is nullable.
        var scalars = ir.AttachedProperties
            .Where(property => property.OwnerName == nameof(ToolTip) && property.Name != "Tip")
            .ToArray();
        Assert.Equal(
            ["BetweenShowDelay", "HorizontalOffset", "IsOpen", "Placement", "ServiceEnabled",
                "ShowDelay", "ShowOnDisabled", "VerticalOffset"],
            scalars.Select(property => property.Name).OrderBy(n => n, StringComparer.Ordinal));
        Assert.All(scalars, property => Assert.False(property.IsNullable));
        Assert.All(scalars, property => Assert.Null(property.StringConverterTypeName));
        Assert.Equal(
            MarshallingKind.I32,
            scalars.Single(property => property.Name == "Placement").Kind);
        Assert.Contains(ir.Enums, projected => projected.FullName == "Avalonia.Controls.PlacementMode");

        // Every previously published attached-property group keeps its own statics interface.
        Assert.Equal(
            ["Avalonia.Host.Com.IAvnCanvasStatics", "Avalonia.Host.Com.IAvnDockPanelStatics",
                "Avalonia.Host.Com.IAvnGridStatics", "Avalonia.Host.Com.IAvnRelativePanelStatics",
                "Avalonia.Host.Com.IAvnToolTipStatics"],
            ir.AttachedProperties
                .Select(property => property.StaticsInterfaceName)
                .Distinct(StringComparer.Ordinal)
                .OrderBy(name => name, StringComparer.Ordinal));
    }

    [Fact]
    public void Projects_solid_brushes_and_the_remaining_chrome_members()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal("Avalonia.Host.Com.IAvnBrush", ir.BrushInterfaceName);
        Assert.Equal(1, ir.BrushAbiVersion);
        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid("Avalonia.Host.Com.IAvnBrush", 1),
            ir.BrushInterfaceIid);

        // Every IBrush member projects as the one brush interface and is always nullable,
        // because a control with no brush reports a null pointer rather than a colour.
        var brushMembers = new[]
        {
            (Type: "IAvnBorder", Member: nameof(Border.Background)),
            (Type: "IAvnBorder", Member: nameof(Border.BorderBrush)),
            (Type: "IAvnPanel", Member: nameof(Panel.Background)),
            (Type: "IAvnTemplatedControl", Member: nameof(TemplatedControl.Background)),
            (Type: "IAvnTemplatedControl", Member: nameof(TemplatedControl.BorderBrush)),
            (Type: "IAvnTemplatedControl", Member: nameof(TemplatedControl.Foreground)),
            (Type: "IAvnTextBlock", Member: nameof(TextBlock.Foreground)),
        };
        Assert.All(brushMembers, entry =>
        {
            var property = Type(ir, entry.Type).Properties
                .Single(candidate => candidate.Name == entry.Member);
            Assert.Equal(MarshallingKind.Brush, property.Kind);
            Assert.Equal("Avalonia.Host.Com.IAvnBrush", property.InterfaceName);
            Assert.Equal(ir.BrushInterfaceIid, property.InterfaceIid);
            Assert.True(property.IsNullable);
            Assert.True(property.CanRead && property.CanWrite);
        });

        var border = Type(ir, "IAvnBorder");
        Assert.Equal(
            MarshallingKind.Thickness,
            border.Properties.Single(p => p.Name == nameof(Border.BorderThickness)).Kind);
        Assert.Equal(
            MarshallingKind.CornerRadius,
            border.Properties.Single(p => p.Name == nameof(Border.CornerRadius)).Kind);

        var templated = Type(ir, "IAvnTemplatedControl");
        Assert.Equal(
            MarshallingKind.F64,
            templated.Properties.Single(p => p.Name == nameof(TemplatedControl.FontSize)).Kind);

        var textBlock = Type(ir, "IAvnTextBlock");
        Assert.Equal(
            MarshallingKind.Thickness,
            textBlock.Properties.Single(p => p.Name == nameof(TextBlock.Padding)).Kind);
        Assert.All(
            textBlock.Properties.Where(p => p.Name is
                nameof(TextBlock.FontWeight) or nameof(TextBlock.TextAlignment)),
            property => Assert.Equal(MarshallingKind.I32, property.Kind));
        Assert.All(
            new[] { "FontWeight", "TextAlignment" },
            enumName => Assert.Contains(ir.Enums, projected => projected.Name == enumName));

        // Gradient/drawing/visual brushes are deliberately out of scope, so nothing else in
        // the object model may claim the brush kind.
        Assert.DoesNotContain(
            ir.Types.SelectMany(type => type.Properties),
            property => property.Kind == MarshallingKind.Brush &&
                property.InterfaceName != "Avalonia.Host.Com.IAvnBrush");
    }

    [Fact]
    public void Chrome_members_are_not_duplicated_onto_derived_interfaces()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        // Background is declared once on Panel and once on TemplatedControl; a derived
        // interface that re-declared it would add a second pair of slots for one property.
        Assert.All(
            new[] { "IAvnGrid", "IAvnCanvas", "IAvnDockPanel", "IAvnStackPanel" },
            name => Assert.DoesNotContain(
                Type(ir, name).Properties,
                property => property.Name == nameof(Panel.Background)));
        Assert.All(
            new[] { "IAvnButton", "IAvnContentControl", "IAvnWindow", "IAvnTextBox" },
            name => Assert.DoesNotContain(
                Type(ir, name).Properties,
                property => property.Name is nameof(TemplatedControl.Background)
                    or nameof(TemplatedControl.Foreground)));

        // ComboBox re-declares HorizontalContentAlignment and VerticalContentAlignment with
        // `new`, so the allowlist keeps them on ContentControl alone.
        Assert.All(
            new[] { "IAvnComboBox", "IAvnButton", "IAvnWindow" },
            name => Assert.DoesNotContain(
                Type(ir, name).Properties,
                property => property.Name is nameof(ContentControl.HorizontalContentAlignment)
                    or nameof(ContentControl.VerticalContentAlignment)));
    }

    [Fact]
    public void Projects_the_completeness_members_onto_the_types_that_declare_them()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var contentControl = Type(ir, "IAvnContentControl");
        Assert.All(
            contentControl.Properties.Where(property => property.Name is
                nameof(ContentControl.HorizontalContentAlignment) or
                nameof(ContentControl.VerticalContentAlignment)),
            property =>
            {
                Assert.Equal(MarshallingKind.I32, property.Kind);
                Assert.True(property.CanRead && property.CanWrite);
            });

        var button = Type(ir, "IAvnButton");
        var clickMode = button.Properties.Single(p => p.Name == nameof(Button.ClickMode));
        Assert.Equal(MarshallingKind.I32, clickMode.Kind);
        Assert.Contains(ir.Enums, projected => projected.Name == nameof(ClickMode));
        Assert.All(
            button.Properties.Where(property => property.Name is
                nameof(Button.IsDefault) or nameof(Button.IsCancel)),
            property =>
            {
                Assert.Equal(MarshallingKind.Bool, property.Kind);
                Assert.True(property.CanRead && property.CanWrite);
            });

        // IsPressed is a read-only direct property: Avalonia raises it from input handling, so
        // it projects a getter and no setter.
        var isPressed = button.Properties.Single(p => p.Name == nameof(Button.IsPressed));
        Assert.Equal(MarshallingKind.Bool, isPressed.Kind);
        Assert.True(isPressed.CanRead);
        Assert.False(isPressed.CanWrite);

        var isThreeState = Type(ir, "IAvnToggleButton").Properties
            .Single(p => p.Name == nameof(ToggleButton.IsThreeState));
        Assert.Equal(MarshallingKind.Bool, isThreeState.Kind);

        var listBox = Type(ir, "IAvnListBox");
        Assert.Equal(
            MarshallingKind.I32,
            listBox.Properties.Single(p => p.Name == nameof(ListBox.SelectionMode)).Kind);
        Assert.Contains(ir.Enums, projected => projected.Name == nameof(SelectionMode));
        Assert.All(
            new[] { nameof(ListBox.SelectAll), nameof(ListBox.UnselectAll) },
            name => Assert.Contains(
                listBox.Methods,
                method => method.Name == name && method.Parameters.Count == 0));

        var comboBox = Type(ir, "IAvnComboBox");
        Assert.All(
            comboBox.Properties.Where(property => property.Name is
                nameof(ComboBox.IsDropDownOpen) or nameof(ComboBox.IsEditable)),
            property => Assert.Equal(MarshallingKind.Bool, property.Kind));
        Assert.Equal(
            MarshallingKind.F64,
            comboBox.Properties.Single(p => p.Name == nameof(ComboBox.MaxDropDownHeight)).Kind);
    }

    [Fact]
    public void Projects_grid_definitions_as_the_length_list_string_grid_already_parses()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var grid = Type(ir, "IAvnGrid");
        Assert.All(
            new[] { nameof(Grid.ColumnDefinitions), nameof(Grid.RowDefinitions) },
            name =>
            {
                var property = grid.Properties.Single(candidate => candidate.Name == name);
                Assert.Equal(MarshallingKind.StringUtf16, property.Kind);
                Assert.True(property.CanRead && property.CanWrite);
                // Never null: an empty grid reports an empty list rather than a null pointer.
                Assert.False(property.IsNullable);
                // The managed type is retained so the emitter converts with the type's own
                // Parse/ToString instead of assigning a string to a definition collection.
                Assert.Equal($"Avalonia.Controls.{name}", property.ManagedTypeName);
                Assert.Null(property.InterfaceName);
                Assert.Null(property.ElementKind);
            });

        // The definition collections are not projected as types or collection interfaces of
        // their own, so nothing new is minted to carry them.
        Assert.DoesNotContain(
            ir.Types,
            type => type.Name is "IAvnColumnDefinitions" or "IAvnRowDefinitions"
                or "IAvnColumnDefinition" or "IAvnRowDefinition");
        Assert.DoesNotContain(
            ir.Types.SelectMany(type => type.Properties),
            property => property.Kind == MarshallingKind.ComCollection &&
                property.ManagedTypeName?.EndsWith("Definitions", StringComparison.Ordinal) == true);
    }

    [Fact]
    public void A_string_override_requires_the_managed_type_to_own_both_halves_of_the_round_trip()
    {
        // Grid's definitions qualify: both declare `static T Parse(string)` and override
        // ToString(), so the projection is a conversion the type itself owns.
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        Assert.Equal(2, Type(ir, "IAvnGrid").Properties
            .Count(property => property.ManagedTypeName is
                "Avalonia.Controls.ColumnDefinitions" or "Avalonia.Controls.RowDefinitions"));

        // A type without that pair is refused rather than projected with a guessed conversion.
        var policy = new ProjectionPolicy
        {
            IncludeTypeNames = [typeof(Panel).FullName!],
            IncludeMembers = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
            {
                [typeof(Panel).FullName!] = [nameof(Panel.Children)],
            },
            MemberOverrides = new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal)
            {
                [$"{typeof(Panel).FullName}.{nameof(Panel.Children)}"] =
                    new() { Kind = MarshallingKind.StringUtf16 },
            },
        };

        var refused = ClrTypeExtractor.Extract([typeof(Panel)], policy);
        Assert.Empty(refused.Types.Single().Properties);
        Assert.Contains(refused.Skipped, skipped =>
            skipped.Member == nameof(Panel.Children) &&
            skipped.Reason.Contains("Parse(string)", StringComparison.Ordinal));
    }

    [Fact]
    public void A_string_override_may_delegate_the_round_trip_to_a_host_side_converter()
    {
        // Image.Source is the reason this exists: IImage is an interface, so it can own neither
        // half of the round trip and the host converter owns both instead.
        var policy = new ProjectionPolicy
        {
            IncludeTypeNames = [typeof(Panel).FullName!],
            IncludeMembers = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
            {
                [typeof(Panel).FullName!] = [nameof(Panel.Children)],
            },
            MemberOverrides = new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal)
            {
                [$"{typeof(Panel).FullName}.{nameof(Panel.Children)}"] = new()
                {
                    Kind = MarshallingKind.StringUtf16,
                    StringConverterTypeName = "Some.Host.Converter",
                    IsNullable = true,
                },
            },
        };

        var ir = ClrTypeExtractor.Extract([typeof(Panel)], policy);
        var property = ir.Types.Single().Properties.Single();
        Assert.Equal(MarshallingKind.StringUtf16, property.Kind);
        Assert.Equal("Some.Host.Converter", property.StringConverterTypeName);
        Assert.True(property.IsNullable);
        Assert.DoesNotContain(ir.Skipped, skipped => skipped.Member == nameof(Panel.Children));
    }

    [Fact]
    public void An_attached_property_override_must_marshal_as_a_string()
    {
        var policy = new ProjectionPolicy
        {
            IncludeTypeNames = [typeof(Control).FullName!, typeof(ToolTip).FullName!],
            IncludeMembers = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
            {
                [typeof(Control).FullName!] = [],
                [typeof(ToolTip).FullName!] = [],
            },
            AttachedProperties = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
            {
                [typeof(ToolTip).FullName!] = ["Tip"],
            },
            AttachedPropertyOverrides = new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal)
            {
                [$"{typeof(ToolTip).FullName}.Tip"] = new() { Kind = MarshallingKind.ComInterface },
            },
        };

        var ir = ClrTypeExtractor.Extract([typeof(Control), typeof(ToolTip)], policy);
        Assert.Empty(ir.AttachedProperties);
        Assert.Contains(ir.Skipped, skipped =>
            skipped.Member == "Tip" && skipped.Reason.Contains("string", StringComparison.Ordinal));
    }

    [Fact]
    public void Wave_c_layout_panels_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.All(
            new[]
            {
                "IAvnWrapPanel", "IAvnUniformGrid", "IAvnRelativePanel", "IAvnViewbox",
                "IAvnFlexPanel",
            },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(8, type.AbiVersion);
                Assert.Equal(
                    ClrTypeExtractor.CreateDeterministicIid(type.FullName, 8),
                    type.Iid);
                Assert.True(type.IsConstructible);
            });
        Assert.All(
            new[] { "IAvnThumb", "IAvnGridSplitter" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(10, type.AbiVersion);
                Assert.True(type.IsConstructible);
            });

        Assert.Equal("Avalonia.Host.Com.IAvnPanel", Type(ir, "IAvnWrapPanel").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnPanel", Type(ir, "IAvnUniformGrid").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnPanel", Type(ir, "IAvnRelativePanel").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnPanel", Type(ir, "IAvnFlexPanel").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", Type(ir, "IAvnViewbox").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnThumb").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnThumb", Type(ir, "IAvnGridSplitter").BaseFullName);
        var thumb = Type(ir, "IAvnThumb");
        Assert.All(
            new[] { "DragStarted", "DragDelta", "DragCompleted" },
            name =>
            {
                var ev = thumb.Events.Single(e => e.Name == name);
                Assert.Equal(EventPayloadKind.Fields, ev.PayloadKind);
                var vector = Assert.Single(ev.Parameters);
                Assert.Equal("Vector", vector.Name);
                Assert.Equal(MarshallingKind.Vector, vector.Kind);
            });

        var wrap = Type(ir, "IAvnWrapPanel");
        Assert.All(
            new[] { "Orientation", "ItemsAlignment" },
            name => Assert.Equal(
                MarshallingKind.I32,
                wrap.Properties.Single(property => property.Name == name).Kind));
        Assert.All(
            new[] { "ItemWidth", "ItemHeight", "ItemSpacing", "LineSpacing" },
            name => Assert.Equal(
                MarshallingKind.F64,
                wrap.Properties.Single(property => property.Name == name).Kind));
        Assert.Contains(ir.Enums, projected => projected.Name == nameof(WrapPanelItemsAlignment));

        var uniform = Type(ir, "IAvnUniformGrid");
        Assert.All(
            new[] { "Rows", "Columns", "FirstColumn" },
            name => Assert.Equal(
                MarshallingKind.I32,
                uniform.Properties.Single(property => property.Name == name).Kind));

        var viewbox = Type(ir, "IAvnViewbox");
        var child = viewbox.Properties.Single(property => property.Name == nameof(Viewbox.Child));
        Assert.Equal(MarshallingKind.ComInterface, child.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", child.InterfaceName);
        Assert.True(child.IsNullable);

        var flex = Type(ir, "IAvnFlexPanel");
        Assert.All(
            new[] { "Direction", "JustifyContent", "AlignItems", "AlignContent", "Wrap" },
            name => Assert.Equal(
                MarshallingKind.I32,
                flex.Properties.Single(property => property.Name == name).Kind));
        Assert.Contains(ir.Enums, projected => projected.Name == nameof(FlexDirection));

        var splitter = Type(ir, "IAvnGridSplitter");
        Assert.All(
            new[] { "ResizeDirection", "ResizeBehavior" },
            name => Assert.Equal(
                MarshallingKind.I32,
                splitter.Properties.Single(property => property.Name == name).Kind));
        Assert.DoesNotContain(
            splitter.Properties,
            property => property.Name == nameof(GridSplitter.PreviewContent));

        var relativeStatics = ir.AttachedProperties
            .Where(property => property.OwnerName == nameof(RelativePanel))
            .Select(property => property.Name)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();
        Assert.Equal(
            [
                "AlignBottomWithPanel", "AlignHorizontalCenterWithPanel", "AlignLeftWithPanel",
                "AlignRightWithPanel", "AlignTopWithPanel", "AlignVerticalCenterWithPanel",
            ],
            relativeStatics);
        Assert.All(
            ir.AttachedProperties.Where(property => property.OwnerName == nameof(RelativePanel)),
            property =>
            {
                Assert.Equal(MarshallingKind.Bool, property.Kind);
                Assert.False(property.IsNullable);
            });
        Assert.DoesNotContain(
            ir.AttachedProperties,
            property => property.Name is "Above" or "Below" or "LeftOf" or "RightOf"
                or "AlignLeftWith" or "Order" or "Grow" or "Shrink" or "Basis");

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_d_button_family_publishes_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal(5, Type(ir, "IAvnMenuFlyout").AbiVersion);
        Assert.Equal(15, Type(ir, "IAvnContextMenu").AbiVersion);
        Assert.All(
            new[]
            {
                "IAvnRepeatButton", "IAvnDropDownButton",
                "IAvnHyperlinkButton",
            },
            name => { var type = Type(ir, name); Assert.Equal(11, type.AbiVersion);
                Assert.True(type.IsConstructible);
            });
        Assert.All(
            new[] { "IAvnSplitButton", "IAvnToggleSplitButton" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(12, type.AbiVersion);
                Assert.True(type.IsConstructible);
            });

        Assert.Equal("Avalonia.Host.Com.IAvnButton", Type(ir, "IAvnRepeatButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnButton", Type(ir, "IAvnDropDownButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnButton", Type(ir, "IAvnHyperlinkButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnSplitButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnSplitButton", Type(ir, "IAvnToggleSplitButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnMenuBase", Type(ir, "IAvnContextMenu").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnPopupFlyoutBase", Type(ir, "IAvnMenuFlyout").BaseFullName);

        var repeat = Type(ir, "IAvnRepeatButton");
        Assert.All(
            new[] { "Interval", "Delay" },
            name => Assert.Equal(
                MarshallingKind.I32,
                repeat.Properties.Single(property => property.Name == name).Kind));

        var hyperlink = Type(ir, "IAvnHyperlinkButton");
        var navigateUri = hyperlink.Properties.Single(property => property.Name == "NavigateUri");
        Assert.Equal(MarshallingKind.StringUtf16, navigateUri.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnUri", navigateUri.StringConverterTypeName);
        Assert.True(navigateUri.IsNullable);

        var split = Type(ir, "IAvnSplitButton");
        Assert.Contains(split.Events, @event => @event.Name == "Click");
        Assert.Contains(split.Properties, property => property.Name == "Flyout");
        Assert.Equal(
            MarshallingKind.Command,
            split.Properties.Single(property => property.Name == "Command").Kind);

        var menuFlyout = Type(ir, "IAvnMenuFlyout");
        var items = menuFlyout.Properties.Single(property => property.Name == "Items");
        Assert.Equal(MarshallingKind.ComCollection, items.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnItemList", items.InterfaceName);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_e_input_controls_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var spinner = Type(ir, "IAvnSpinner");
        Assert.Equal(9, spinner.AbiVersion);
        Assert.False(spinner.IsConstructible);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", spinner.BaseFullName);

        Assert.All(
            new[] { "IAvnButtonSpinner" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(9, type.AbiVersion);
                Assert.True(type.IsConstructible);
            });
        Assert.Equal(11, Type(ir, "IAvnSelectableTextBlock").AbiVersion);
        Assert.True(Type(ir, "IAvnSelectableTextBlock").IsConstructible);
        Assert.Equal(10, Type(ir, "IAvnNumericUpDown").AbiVersion);
        Assert.Equal(16, Type(ir, "IAvnAutoCompleteBox").AbiVersion);
        Assert.Equal(15, Type(ir, "IAvnMaskedTextBox").AbiVersion);

        Assert.Equal("Avalonia.Host.Com.IAvnSpinner", Type(ir, "IAvnButtonSpinner").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnNumericUpDown").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTextBox", Type(ir, "IAvnMaskedTextBox").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTextBlock", Type(ir, "IAvnSelectableTextBlock").BaseFullName);

        var value = Type(ir, "IAvnNumericUpDown").Properties.Single(p => p.Name == "Value");
        Assert.Equal(MarshallingKind.StringUtf16, value.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnDecimal", value.StringConverterTypeName);
        Assert.True(value.IsNullable);
        var minimum = Type(ir, "IAvnNumericUpDown").Properties.Single(p => p.Name == "Minimum");
        Assert.Equal("Avalonia.Host.Com.AvnDecimalValue", minimum.StringConverterTypeName);
        Assert.False(minimum.IsNullable);

        var selectedText = Type(ir, "IAvnSelectableTextBlock").Properties
            .Single(p => p.Name == "SelectedText");
        Assert.True(selectedText.CanRead);
        Assert.False(selectedText.CanWrite);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_f_calendar_family_publishes_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.All(
            new[] { "IAvnCalendar", "IAvnCalendarDatePicker" },
            name =>
            {
                var type = Type(ir, name);
                Assert.True(type.IsConstructible);
                Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", type.BaseFullName);
            });
        // U17 grew Calendar alone (DisplayDateChanged); the picker did not move.
        Assert.Equal(12, Type(ir, "IAvnCalendar").AbiVersion);
        Assert.Equal(11, Type(ir, "IAvnCalendarDatePicker").AbiVersion);

        var selected = Type(ir, "IAvnCalendar").Properties.Single(p => p.Name == "SelectedDate");
        Assert.Equal(MarshallingKind.StringUtf16, selected.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnCalendarDate", selected.StringConverterTypeName);
        Assert.True(selected.IsNullable);

        var display = Type(ir, "IAvnCalendar").Properties.Single(p => p.Name == "DisplayDate");
        Assert.Equal("Avalonia.Host.Com.AvnCalendarDateValue", display.StringConverterTypeName);
        Assert.False(display.IsNullable);

        Assert.Contains(ir.Enums, e => e.Name == nameof(CalendarMode));
        Assert.Contains(ir.Enums, e => e.Name == nameof(CalendarSelectionMode));
        // U23: SelectedDates and BlackoutDates cross as the host-implemented
        // IAvnDateTimeList over the calendar's own collections.
        var selectedDates = Type(ir, "IAvnCalendar").Properties.Single(p => p.Name == "SelectedDates");
        Assert.Equal(MarshallingKind.ComCollection, selectedDates.Kind);
        Assert.Equal(MarshallingKind.DateTimeI64, selectedDates.ElementKind);
        Assert.Equal("Avalonia.Host.Com.AvnDateList", selectedDates.HostImplementationTypeName);
        Assert.False(selectedDates.CanWrite);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_g_content_chrome_publishes_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal("Avalonia.Host.Com.IAvnSelectingItemsControl", Type(ir, "IAvnCarousel").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnTransitioningContentControl").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnLabel").BaseFullName);
        Assert.Equal(10, Type(ir, "IAvnLabel").AbiVersion);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnSeparator").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnHeaderedContentControl", Type(ir, "IAvnGroupBox").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnUserControl").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnDecorator", Type(ir, "IAvnLayoutTransformControl").BaseFullName);

        var isSwiping = Type(ir, "IAvnCarousel").Properties.Single(p => p.Name == "IsSwiping");
        Assert.True(isSwiping.CanRead);
        Assert.False(isSwiping.CanWrite);
        Assert.DoesNotContain(Type(ir, "IAvnCarousel").Properties, p => p.Name == "PageTransition");
        var labelTarget = Type(ir, "IAvnLabel").Properties.Single(p => p.Name == "Target");
        Assert.Equal(MarshallingKind.ComInterface, labelTarget.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", labelTarget.InterfaceName);
        Assert.True(labelTarget.IsNullable);
        Assert.DoesNotContain(
            Type(ir, "IAvnLayoutTransformControl").Properties,
            p => p.Name == "LayoutTransform");

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_h_shapes_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var shape = Type(ir, "IAvnShape");
        Assert.Equal(8, shape.AbiVersion);
        Assert.False(shape.IsConstructible);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", shape.BaseFullName);
        Assert.All(
            new[] { "Fill", "Stroke" },
            name => Assert.Equal(
                MarshallingKind.Brush,
                shape.Properties.Single(p => p.Name == name).Kind));

        Assert.All(
            new[] { "IAvnRectangle", "IAvnEllipse", "IAvnLine", "IAvnPath", "IAvnPolygon",
                "IAvnPolyline", "IAvnArc", "IAvnSector" },
            name =>
            {
                var type = Type(ir, name);
                Assert.Equal(8, type.AbiVersion);
                Assert.True(type.IsConstructible);
                Assert.Equal("Avalonia.Host.Com.IAvnShape", type.BaseFullName);
            });

        var start = Type(ir, "IAvnLine").Properties.Single(p => p.Name == "StartPoint");
        Assert.Equal(MarshallingKind.Point, start.Kind);

        var data = Type(ir, "IAvnPath").Properties.Single(p => p.Name == "Data");
        Assert.Equal(MarshallingKind.StringUtf16, data.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnGeometry", data.StringConverterTypeName);
        Assert.True(data.IsNullable);
        // U22: Points crosses as a UTF-16 "x,y x,y" list through the
        // AvnPointList converter.
        var polygonPoints = Type(ir, "IAvnPolygon").Properties.Single(p => p.Name == "Points");
        Assert.Equal(MarshallingKind.StringUtf16, polygonPoints.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnPointList", polygonPoints.StringConverterTypeName);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_i_overlay_publishes_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal("Avalonia.Host.Com.IAvnControl", Type(ir, "IAvnPopup").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnAvaloniaObject", Type(ir, "IAvnTrayIcon").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnWindowNotificationManager").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnNotificationCard").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnRefreshContainer").BaseFullName);

        var child = Type(ir, "IAvnPopup").Properties.Single(p => p.Name == "Child");
        Assert.Equal(MarshallingKind.ComInterface, child.Kind);
        // U21 projects the tray icon's Icon as a write-oriented path string;
        // Menu (NativeMenu) stays a gap.
        var trayIconIcon = Type(ir, "IAvnTrayIcon").Properties.Single(p => p.Name == "Icon");
        Assert.Equal(MarshallingKind.StringUtf16, trayIconIcon.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnWindowIcon", trayIconIcon.StringConverterTypeName);
        Assert.DoesNotContain(Type(ir, "IAvnTrayIcon").Properties, p => p.Name == "Menu");
        Assert.Equal(
            MarshallingKind.Command,
            Type(ir, "IAvnTrayIcon").Properties.Single(p => p.Name == "Command").Kind);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_j_command_bar_publishes_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnCommandBar").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnButton", Type(ir, "IAvnCommandBarButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnToggleButton", Type(ir, "IAvnCommandBarToggleButton").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnSeparator", Type(ir, "IAvnCommandBarSeparator").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", Type(ir, "IAvnPipsPager").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnDecorator", Type(ir, "IAvnThemeVariantScope").BaseFullName);

        var content = Type(ir, "IAvnCommandBar").Properties.Single(p => p.Name == "Content");
        Assert.Equal(MarshallingKind.ComInterface, content.Kind);
        // U30 projects the command lists as a live control-list adapter.
        var primary = Type(ir, "IAvnCommandBar").Properties.Single(p => p.Name == "PrimaryCommands");
        Assert.Equal(MarshallingKind.ComCollection, primary.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControlList", primary.InterfaceName);
        var buttonIcon = Type(ir, "IAvnCommandBarButton").Properties.Single(p => p.Name == "Icon");
        Assert.Equal(MarshallingKind.Variant, buttonIcon.Kind);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_k_icons_and_table_view_publish_new_interfaces_at_version_one()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var icon = Type(ir, "IAvnIconElement");
        Assert.False(icon.IsConstructible);
        Assert.Equal("Avalonia.Host.Com.IAvnTemplatedControl", icon.BaseFullName);

        var pathIcon = Type(ir, "IAvnPathIcon");
        Assert.True(pathIcon.IsConstructible);
        Assert.Equal("Avalonia.Host.Com.IAvnIconElement", pathIcon.BaseFullName);
        var data = pathIcon.Properties.Single(p => p.Name == "Data");
        Assert.Equal(MarshallingKind.StringUtf16, data.Kind);

        Assert.Equal("Avalonia.Host.Com.IAvnListBox", Type(ir, "IAvnTableView").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnStyledElement", Type(ir, "IAvnTableViewColumn").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnListBoxItem", Type(ir, "IAvnTableViewRow").BaseFullName);
        Assert.Equal("Avalonia.Host.Com.IAvnContentControl", Type(ir, "IAvnTableViewCell").BaseFullName);

        // U33 projects the column list through the live control-list adapter.
        var columns = Type(ir, "IAvnTableView").Properties.Single(p => p.Name == "Columns");
        Assert.Equal(MarshallingKind.ComCollection, columns.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControlList", columns.InterfaceName);

        var width = Type(ir, "IAvnTableViewColumn").Properties.Single(p => p.Name == "Width");
        Assert.Equal(MarshallingKind.StringUtf16, width.Kind);

        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_l_window_chrome_bumps_only_window()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var window = Type(ir, "IAvnWindow");
        Assert.Equal(18, window.AbiVersion);
        Assert.Contains(window.Methods, m => m.Name == "Hide");
        Assert.All(
            new[]
            {
                "SizeToContent", "ShowActivated", "ShowInTaskbar", "CanMinimize",
                "CanMaximize", "WindowStartupLocation", "WindowDecorations", "ClosingBehavior",
                "ExtendClientAreaToDecorationsHint", "ExtendClientAreaTitleBarHeightHint",
                "IsExtendedIntoWindowDecorations", "WindowDecorationMargin", "OffScreenMargin",
                "IsDialog",
            },
            name => Assert.Contains(window.Properties, p => p.Name == name));
        // U17: Position crosses as a blittable AvnPixelPoint; U21 projects Icon
        // as a write-oriented UTF-16 path through the AvnWindowIcon converter.
        var position = window.Properties.Single(p => p.Name == "Position");
        Assert.Equal(MarshallingKind.PixelPointI32, position.Kind);
        Assert.True(position.CanWrite);
        var icon = window.Properties.Single(p => p.Name == "Icon");
        Assert.Equal(MarshallingKind.StringUtf16, icon.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnWindowIcon", icon.StringConverterTypeName);
        var closing = window.Events.Single(e => e.Name == "Closing");
        Assert.Equal(EventPayloadKind.Fields, closing.PayloadKind);
        Assert.Contains(closing.Parameters, p => p.Name == "Cancel" && p.Direction == ParameterDirection.InOut);
        Assert.Contains(closing.Parameters, p => p.Name == "CloseReason" && p.Kind == MarshallingKind.I32);
        Assert.Contains(closing.Parameters, p => p.Name == "IsProgrammatic" && p.Kind == MarshallingKind.Bool);
        Assert.Equal(13, Type(ir, "IAvnContentControl").AbiVersion);
    }

    [Fact]
    public void Wave_m_fonts_bump_templated_control_and_text_block()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var templated = Type(ir, "IAvnTemplatedControl");
        Assert.Equal(12, templated.AbiVersion);
        Assert.All(
            new[] { "FontFamily", "FontStyle", "FontWeight", "FontStretch", "LetterSpacing", "Padding" },
            name => Assert.Contains(templated.Properties, p => p.Name == name));
        var fontFamily = templated.Properties.Single(p => p.Name == "FontFamily");
        Assert.Equal(MarshallingKind.StringUtf16, fontFamily.Kind);
        Assert.Equal("Avalonia.Media.FontFamily", fontFamily.ManagedTypeName);
        Assert.Null(fontFamily.StringConverterTypeName);

        var textBlock = Type(ir, "IAvnTextBlock");
        Assert.Equal(14, textBlock.AbiVersion);
        Assert.All(
            new[] { "FontFamily", "FontStyle", "FontStretch", "Background", "LetterSpacing",
                "LineSpacing", "MaxLines", "TextWrapping" },
            name => Assert.Contains(textBlock.Properties, p => p.Name == name));

        Assert.Equal(11, Type(ir, "IAvnSelectableTextBlock").AbiVersion);
        Assert.Equal(12, Type(ir, "IAvnBorder").AbiVersion);
        var trimming = textBlock.Properties.Single(p => p.Name == "TextTrimming");
        Assert.Equal(MarshallingKind.StringUtf16, trimming.Kind);
        Assert.Equal("Avalonia.Host.Com.AvnTextTrimming", trimming.StringConverterTypeName);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_n_textbox_remainder_bumps_only_textbox_and_masked_textbox()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var textBox = Type(ir, "IAvnTextBox");
        Assert.Equal(18, textBox.AbiVersion);
        Assert.All(
            new[] { "SelectedText", "TextAlignment", "SelectionBrush", "InnerLeftContent",
                "UseFloatingPlaceholder", "PlaceholderForeground" },
            name => Assert.Contains(textBox.Properties, p => p.Name == name));
        Assert.All(
            new[] { "SelectAll", "ClearSelection" },
            name => Assert.Contains(textBox.Methods, m => m.Name == name));
        // U17: CaretBlinkInterval crosses as its tick count; the obsolete Watermark
        // alias of PlaceholderText stays a gap.
        Assert.Equal(
            MarshallingKind.TimeSpanI64,
            textBox.Properties.Single(p => p.Name == "CaretBlinkInterval").Kind);
        Assert.DoesNotContain(textBox.Properties, p => p.Name == "Watermark");
        Assert.Equal(15, Type(ir, "IAvnMaskedTextBox").AbiVersion);
        Assert.Contains(Type(ir, "IAvnMaskedTextBox").Properties, p => p.Name == "MaskCompleted");
        Assert.Equal(12, Type(ir, "IAvnTemplatedControl").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_o_combo_and_scroll_use_vector_and_size()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var combo = Type(ir, "IAvnComboBox");
        Assert.Equal(17, combo.AbiVersion);
        Assert.Contains(combo.Properties, p => p.Name == "Text");
        Assert.Contains(combo.Methods, m => m.Name == "Clear");
        Assert.Contains(combo.Events, e => e.Name == "DropDownOpened");

        var scroll = Type(ir, "IAvnScrollViewer");
        Assert.Equal(15, scroll.AbiVersion);
        Assert.Equal(MarshallingKind.Size, scroll.Properties.Single(p => p.Name == "Extent").Kind);
        Assert.Equal(MarshallingKind.Vector, scroll.Properties.Single(p => p.Name == "Offset").Kind);
        Assert.Equal(MarshallingKind.Size, scroll.Properties.Single(p => p.Name == "Viewport").Kind);
        Assert.Equal(MarshallingKind.Vector, scroll.Properties.Single(p => p.Name == "ScrollBarMaximum").Kind);
        Assert.False(scroll.Properties.Single(p => p.Name == "Extent").CanWrite);
        Assert.True(scroll.Properties.Single(p => p.Name == "Offset").CanWrite);
        Assert.Equal(13, Type(ir, "IAvnContentControl").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_p_projects_instance_flyout_and_menu_item_open()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var button = Type(ir, "IAvnButton");
        Assert.Equal(16, button.AbiVersion);
        var flyout = button.Properties.Single(p => p.Name == "Flyout");
        Assert.Equal(MarshallingKind.ComInterface, flyout.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnFlyoutBase", flyout.InterfaceName);
        Assert.True(flyout.IsNullable);

        var split = Type(ir, "IAvnSplitButton");
        Assert.Equal(12, split.AbiVersion);
        Assert.Contains(split.Properties, p => p.Name == "Flyout");

        var menu = Type(ir, "IAvnMenuItem");
        Assert.Equal(15, menu.AbiVersion);
        Assert.Contains(menu.Properties, p => p.Name == "HasSubMenu");
        Assert.Contains(menu.Methods, m => m.Name == "Open");
        Assert.Contains(menu.Methods, m => m.Name == "Close");
        Assert.Equal(13, Type(ir, "IAvnContentControl").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Wave_q_sweeps_leaf_input_scalars()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        Assert.Equal(16, Type(ir, "IAvnAutoCompleteBox").AbiVersion);
        Assert.Contains(Type(ir, "IAvnAutoCompleteBox").Properties, p => p.Name == "SearchText");
        Assert.Equal(12, Type(ir, "IAvnCalendar").AbiVersion);
        Assert.Contains(Type(ir, "IAvnCalendar").Properties, p => p.Name == "IsWeekNumberVisible");
        Assert.Equal(11, Type(ir, "IAvnCalendarDatePicker").AbiVersion);
        Assert.Contains(Type(ir, "IAvnCalendarDatePicker").Methods, m => m.Name == "Clear");
        Assert.Equal(10, Type(ir, "IAvnNumericUpDown").AbiVersion);
        Assert.Contains(Type(ir, "IAvnNumericUpDown").Properties, p => p.Name == "TextAlignment");
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Items_control_leftovers_keep_items_and_selection()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var items = Type(ir, "IAvnItemsControl");
        Assert.Equal(15, items.AbiVersion);
        Assert.Contains(items.Properties, p => p.Name == "Items");
        Assert.Contains(items.Properties, p => p.Name == "ItemCount");
        Assert.Contains(items.Methods, m => m.ManagedName == "ScrollIntoView");
        var selecting = Type(ir, "IAvnSelectingItemsControl");
        Assert.Equal(15, selecting.AbiVersion);
        Assert.Contains(selecting.Properties, p => p.Name == "SelectedIndex");
        Assert.Contains(selecting.Events, e => e.Name == "SelectionChanged");
        Assert.Contains(selecting.Properties, p => p.Name == "AutoScrollToSelectedItem");
        Assert.Equal(13, Type(ir, "IAvnContentControl").AbiVersion);
        Assert.Equal(18, Type(ir, "IAvnWindow").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Text_leftovers_project_line_metrics_and_scroll_to_line()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var textBlock = Type(ir, "IAvnTextBlock");
        Assert.Equal(14, textBlock.AbiVersion);
        Assert.Contains(textBlock.Properties, p => p.Name == "LineHeight");
        Assert.Contains(textBlock.Properties, p => p.Name == "BaselineOffset");
        Assert.Contains(textBlock.Properties, p => p.Name == "TextTrimming");

        var selectable = Type(ir, "IAvnSelectableTextBlock");
        Assert.Equal(11, selectable.AbiVersion);
        Assert.Contains(selectable.Properties, p => p.Name == "SelectionBrush");
        Assert.Contains(selectable.Methods, m => m.Name == "SelectAll");
        Assert.Contains(selectable.Events, e => e.Name == "CopyingToClipboard");

        var textBox = Type(ir, "IAvnTextBox");
        Assert.Equal(18, textBox.AbiVersion);
        Assert.Contains(textBox.Methods, m => m.ManagedName == "ScrollToLine");
        var lineCount = textBox.Methods.Single(m => m.ManagedName == "GetLineCount");
        Assert.Contains(lineCount.Parameters, p => p.Name == "value" && p.Direction == ParameterDirection.Out);
        Assert.Equal(MarshallingKind.CharUtf16, textBox.Properties.Single(p => p.Name == "PasswordChar").Kind);
        Assert.Equal(15, Type(ir, "IAvnMaskedTextBox").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Leaf_leftovers_project_marshallable_scalars_and_commands()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        Assert.Equal(12, Type(ir, "IAvnCommandBar").AbiVersion);
        Assert.Contains(Type(ir, "IAvnCommandBar").Properties, p => p.Name == "HasSecondaryCommands");
        Assert.Contains(Type(ir, "IAvnCommandBar").Events, e => e.Name == "Opened");
        Assert.Equal(13, Type(ir, "IAvnCarousel").AbiVersion);
        Assert.Contains(Type(ir, "IAvnCarousel").Methods, m => m.Name == "Next");
        Assert.Equal(17, Type(ir, "IAvnComboBox").AbiVersion);
        Assert.Equal(11, Type(ir, "IAvnDatePicker").AbiVersion);
        Assert.Contains(Type(ir, "IAvnDatePicker").Properties, p => p.Name == "VerticalContentAlignment");
        Assert.Equal(15, Type(ir, "IAvnContextMenu").AbiVersion);
        Assert.Equal(13, Type(ir, "IAvnProgressBar").AbiVersion);
        Assert.Contains(Type(ir, "IAvnProgressBar").Properties, p => p.Name == "Percentage");
        Assert.Equal(12, Type(ir, "IAvnStackPanel").AbiVersion);
        Assert.Equal(12, Type(ir, "IAvnBorder").AbiVersion);
        Assert.Contains(Type(ir, "IAvnBorder").Properties, p => p.Name == "ClipToBoundsRadius");
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Overlay_chrome_projects_popup_open_close_and_placement_target()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var popup = Type(ir, "IAvnPopup");
        Assert.Equal(8, popup.AbiVersion);
        Assert.Contains(popup.Methods, m => m.Name == "Open");
        Assert.Contains(popup.Methods, m => m.Name == "Close");
        Assert.Contains(popup.Events, e => e.Name == "Opened");
        var target = popup.Properties.Single(p => p.Name == "PlacementTarget");
        Assert.Equal(MarshallingKind.ComInterface, target.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", target.InterfaceName);
        var placementRect = popup.Properties.Single(p => p.Name == "PlacementRect");
        Assert.Equal(MarshallingKind.Rect, placementRect.Kind);
        Assert.True(placementRect.IsNullable);
        Assert.Contains(popup.Properties, p => p.Name == "PlacementAnchor");

        var flyoutBase = Type(ir, "IAvnPopupFlyoutBase");
        Assert.Equal(6, flyoutBase.AbiVersion);
        var popupProp = flyoutBase.Properties.Single(p => p.Name == "Popup");
        Assert.Equal("Avalonia.Host.Com.IAvnPopup", popupProp.InterfaceName);

        Assert.Equal(15, Type(ir, "IAvnContextMenu").AbiVersion);
        Assert.Contains(Type(ir, "IAvnContextMenu").Properties, p => p.Name == "PlacementTarget");
        var openWithControl = Type(ir, "IAvnContextMenu").Methods
            .Single(m => m.Name == "OpenWithControl");
        Assert.Equal(MarshallingKind.ComInterface, openWithControl.Parameters.Single().Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnControl", openWithControl.Parameters.Single().InterfaceName);
        Assert.True(openWithControl.Parameters.Single().IsNullable);
        Assert.Contains(Type(ir, "IAvnContextMenu").Events, e => e.Name == "Opening");
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Control_leftovers_project_context_menu_and_loaded()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var control = Type(ir, "IAvnControl");
        Assert.Equal(8, control.AbiVersion);
        var menu = control.Properties.Single(p => p.Name == "ContextMenu");
        Assert.Equal(MarshallingKind.ComInterface, menu.Kind);
        Assert.Equal("Avalonia.Host.Com.IAvnContextMenu", menu.InterfaceName);
        Assert.True(menu.IsNullable);
        var flyout = control.Properties.Single(p => p.Name == "ContextFlyout");
        Assert.Equal("Avalonia.Host.Com.IAvnFlyoutBase", flyout.InterfaceName);
        Assert.Contains(control.Properties, p => p.Name == "IsLoaded");
        Assert.Contains(control.Events, e => e.Name == "Loaded");
        Assert.Contains(control.Events, e => e.Name == "Unloaded");
        Assert.Equal(6, Type(ir, "IAvnStyledElement").AbiVersion);
        Assert.Equal(16, Type(ir, "IAvnButton").AbiVersion);
        Assert.Equal(4, Type(ir, "IAvnFlyout").AbiVersion);
        Assert.Equal(13, ir.FactoryAbiVersion);
    }

    [Fact]
    public void Produces_stable_unique_iids_and_explicit_gap_report()    {
        var first = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var second = ClrTypeExtractor.Extract(KernelTypes.Reverse(), AvaloniaProjectionProfiles.ObjectModelKernel);

        Assert.Equal(
            first.Types.Select(t => (t.FullName, t.Iid)),
            second.Types.Select(t => (t.FullName, t.Iid)));
        Assert.Equal(first.Types.Count, first.Types.Select(t => t.Iid).Distinct().Count());
        Assert.DoesNotContain(first.Skipped, s =>
            s.Owner == typeof(Button).FullName && s.Member == nameof(Button.Command));
        Assert.Equal(
            MarshallingKind.Command,
            Type(first, "IAvnButton").Properties.Single(p => p.Name == nameof(Button.Command)).Kind);
    }

    [Fact]
    public void Iids_are_versioned_per_interface_not_by_ir_schema()
    {
        const string name = "Avalonia.Host.Com.IAvnButton";

        Assert.Equal(
            ClrTypeExtractor.CreateDeterministicIid(name, 1),
            ClrTypeExtractor.CreateDeterministicIid(name));
        Assert.NotEqual(
            ClrTypeExtractor.CreateDeterministicIid(name, 1),
            ClrTypeExtractor.CreateDeterministicIid(name, 2));
    }

    private static ProjectedType Type(ProjectionIr ir, string name) =>
        ir.Types.Single(t => t.Name == name);
}



