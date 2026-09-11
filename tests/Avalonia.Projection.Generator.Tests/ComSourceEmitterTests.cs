using System;
using System.IO;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Projection.Generator;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

public class ComSourceEmitterTests
{
    /// <summary>Every type the object-model kernel projects, in the order the profile lists them.</summary>
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
    public void Emits_fixture_interfaces_with_stable_guids_and_preserve_sig()
    {
        var ir = ComInterfaceExtractor.Extract(typeof(IAvnEcho).Assembly, new ProjectionPolicy
        {
            IncludeTypeNames = [typeof(IAvnEcho).FullName!, typeof(IAvnActivationFactory).FullName!],
        });
        var files = ComSourceEmitter.Emit(ir);

        Assert.True(files.ContainsKey("IAvnEcho.g.cs"));
        Assert.True(files.ContainsKey("IAvnActivationFactory.g.cs"));

        var echo = files["IAvnEcho.g.cs"];
        Assert.Contains("[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]", echo, StringComparison.Ordinal);
        Assert.Contains("[Guid(\"6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D11\")]", echo, StringComparison.Ordinal);
        Assert.Contains("[PreserveSig]", echo, StringComparison.Ordinal);
        Assert.Contains("int Ping(int value, out int result);", echo, StringComparison.Ordinal);
        Assert.Contains("int EchoString(string? input, out string? output);", echo, StringComparison.Ordinal);
        Assert.Contains("int Fail();", echo, StringComparison.Ordinal);

        var factory = files["IAvnActivationFactory.g.cs"];
        Assert.Contains("[Guid(\"6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D10\")]", factory, StringComparison.Ordinal);
        Assert.Contains("int CreateEcho(out IAvnEcho? echo);", factory, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_generalized_kernel_interfaces_wrappers_runtime_and_factory()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);

        var files = ComSourceEmitter.Emit(ir);

        var button = files["IAvnButton.g.cs"];
        Assert.Contains("public partial interface IAvnButton : IAvnContentControl", button, StringComparison.Ordinal);
        Assert.Contains("public sealed partial class AvnButton : IAvnButton", button, StringComparison.Ordinal);
        Assert.Contains("public int SetContent(IAvnControl? value)", button, StringComparison.Ordinal);
        Assert.Contains("_value.Content = (global::System.Object)ProjectionRuntime.Unwrap(value)!;", button, StringComparison.Ordinal);
        Assert.Contains("int AdviseClick(IAvnButtonClickHandler? handler, out long subscriptionId);", button, StringComparison.Ordinal);
        Assert.Contains("eventSource.Click += callback;", button, StringComparison.Ordinal);
        Assert.Contains("eventSource.Click -= callback", button, StringComparison.Ordinal);

        var control = files["IAvnControl.g.cs"];
        Assert.Contains("int SetWidth(double value);", control, StringComparison.Ordinal);
        Assert.Contains("int SetHeight(double value);", control, StringComparison.Ordinal);
        Assert.Contains(
            "handler.Invoke((int)eventArgs.Key, (int)eventArgs.PhysicalKey, (int)eventArgs.KeyModifiers, eventArgs.KeySymbol, ref handled)",
            control,
            StringComparison.Ordinal);
        Assert.Contains("eventArgs.Handled = handled != 0;", control, StringComparison.Ordinal);
        Assert.Contains("int AdvisePointerEntered(IAvnControlPointerEnteredHandler? handler", control, StringComparison.Ordinal);
        Assert.Contains("int AdvisePointerExited(IAvnControlPointerExitedHandler? handler", control, StringComparison.Ordinal);
        Assert.Contains(
            "int Invoke(int Key, int PhysicalKey, int KeyModifiers, string? KeySymbol, ref int Handled);",
            files["IAvnControlKeyDownHandler.g.cs"],
            StringComparison.Ordinal);

        var clickHandler = files["IAvnButtonClickHandler.g.cs"];
        Assert.Contains("public partial interface IAvnButtonClickHandler", clickHandler, StringComparison.Ordinal);
        Assert.Contains("int Invoke();", clickHandler, StringComparison.Ordinal);

        var window = files["IAvnWindow.g.cs"];
        Assert.Contains("int GetTitle(out string? value);", window, StringComparison.Ordinal);
        Assert.Contains("public int Show()", window, StringComparison.Ordinal);
        Assert.Contains("_value.Show();", window, StringComparison.Ordinal);

        var toggleSwitch = files["IAvnToggleSwitch.g.cs"];
        Assert.Contains("public partial interface IAvnToggleSwitch : IAvnToggleButton", toggleSwitch, StringComparison.Ordinal);
        Assert.Contains("int SetOnContent(IAvnControl? value);", toggleSwitch, StringComparison.Ordinal);
        Assert.Contains("int SetOffContent(IAvnControl? value);", toggleSwitch, StringComparison.Ordinal);

        var expander = files["IAvnExpander.g.cs"];
        Assert.Contains("public partial interface IAvnExpander : IAvnHeaderedContentControl", expander, StringComparison.Ordinal);
        Assert.Contains("int AdviseExpanded(IAvnExpanderExpandedHandler? handler", expander, StringComparison.Ordinal);

        var runtime = files["ProjectionRuntime.g.cs"];
        Assert.Contains("global::Avalonia.Controls.Button typed => new AvnButton(typed)", runtime, StringComparison.Ordinal);
        Assert.Contains("ConditionalWeakTable<global::Avalonia.AvaloniaObject, IAvnAvaloniaObject>", runtime, StringComparison.Ordinal);
        Assert.Contains("internal static int TrackedObjectIdCount => s_objects.Count;", runtime, StringComparison.Ordinal);
        Assert.Contains("internal static int LiveManagedObjectCount", runtime, StringComparison.Ordinal);

        var factory = files["IAvnControlFactory.g.cs"];
        Assert.Contains("int CreateButton(out IAvnButton? value);", factory, StringComparison.Ordinal);
        Assert.Contains("int CreateWindow(out IAvnWindow? value);", factory, StringComparison.Ordinal);

        var list = files["IAvnControlList.g.cs"];
        Assert.Contains("int GetAt(int index, out IAvnControl? value);", list, StringComparison.Ordinal);
        Assert.Contains("int Add(IAvnControl? value);", list, StringComparison.Ordinal);
        // The control list is host-implemented: the generated file carries the
        // interface and the marshal routes through AvnControlList.
        Assert.Contains("public static class AvnControlListMarshal", list, StringComparison.Ordinal);
        Assert.Contains("AvnControlList.FromManaged(value);", list, StringComparison.Ordinal);

        var items = files["IAvnItemList.g.cs"];
        Assert.Contains("private readonly global::Avalonia.Controls.ItemCollection _value;", items, StringComparison.Ordinal);
        Assert.Contains("ProjectionRuntime.Wrap((global::Avalonia.Controls.Control)_value[index]!)", items, StringComparison.Ordinal);

        var roots = files["ProjectionAotRoots.g.cs"];
        Assert.Contains("typeof(AvnButton)", roots, StringComparison.Ordinal);
        Assert.Contains("typeof(AvnControlList)", roots, StringComparison.Ordinal);
        Assert.Contains("typeof(AvnWindow)", roots, StringComparison.Ordinal);
        Assert.Contains(
            "global::Avalonia.Host.ProjectionDiagnostics.WrapperCreated();",
            button,
            StringComparison.Ordinal);
        Assert.Contains(
            "global::Avalonia.Host.ProjectionDiagnostics.SubscriptionAdded();",
            button,
            StringComparison.Ordinal);

        var gridStatics = files["IAvnGridStatics.g.cs"];
        Assert.Contains("int SetRow(IAvnControl? target, int value);", gridStatics, StringComparison.Ordinal);

        var stringList = files["IAvnStringList.g.cs"];
        Assert.Contains("int Add(string value);", stringList, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_grid_definitions_as_strings_converted_by_the_managed_types_own_parse()
    {
        var ir = ClrTypeExtractor.Extract(
            [typeof(AvaloniaObject), typeof(StyledElement), typeof(Control), typeof(Panel), typeof(Grid)],
            AvaloniaProjectionProfiles.ObjectModelKernel);
        var grid = ComSourceEmitter.Emit(ir)["IAvnGrid.g.cs"];

        // The ABI slot is a plain non-nullable UTF-16 string in both directions.
        foreach (var expected in new[]
                 {
                     "int GetColumnDefinitions(out string value);",
                     "int SetColumnDefinitions(string value);",
                     "int GetRowDefinitions(out string value);",
                     "int SetRowDefinitions(string value);",
                 })
        {
            Assert.Contains(expected, grid, StringComparison.Ordinal);
        }

        // The wrapper converts with ColumnDefinitions/RowDefinitions' own round trip rather
        // than assigning a string straight onto a definition collection.
        foreach (var expected in new[]
                 {
                     "value = _value.ColumnDefinitions.ToString();",
                     "_value.ColumnDefinitions = global::Avalonia.Controls.ColumnDefinitions.Parse(value);",
                     "value = _value.RowDefinitions.ToString();",
                     "_value.RowDefinitions = global::Avalonia.Controls.RowDefinitions.Parse(value);",
                 })
        {
            Assert.Contains(expected, grid, StringComparison.Ordinal);
        }

        // No definition object crosses, so no interface is minted for one.
        Assert.DoesNotContain("IAvnColumnDefinition", grid, StringComparison.Ordinal);
        Assert.DoesNotContain("IAvnRowDefinition", grid, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_image_source_as_a_string_converted_by_the_host_side_converter()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var image = ComSourceEmitter.Emit(ir)["IAvnImage.g.cs"];

        Assert.Contains("public partial interface IAvnImage : IAvnControl", image, StringComparison.Ordinal);
        // The slot is a nullable string, because an image with no source is not an empty path.
        Assert.Contains("int GetSource(out string? value);", image, StringComparison.Ordinal);
        Assert.Contains("int SetSource(string? value);", image, StringComparison.Ordinal);
        // Neither half touches IImage: the host converter owns the whole conversion.
        Assert.Contains(
            "value = global::Avalonia.Host.Com.AvnImageSource.ToAbi(_value.Source);",
            image,
            StringComparison.Ordinal);
        Assert.Contains(
            "_value.Source = global::Avalonia.Host.Com.AvnImageSource.FromAbi(value);",
            image,
            StringComparison.Ordinal);
        Assert.DoesNotContain("_value.Source.ToString()", image, StringComparison.Ordinal);
        Assert.DoesNotContain("global::Avalonia.Media.IImage.Parse", image, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_tool_tip_statics_with_a_nullable_string_tip()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var files = ComSourceEmitter.Emit(ir);
        var statics = files["IAvnToolTipStatics.g.cs"];

        Assert.Contains(
            "int GetTip(IAvnControl? target, out string? value);",
            statics,
            StringComparison.Ordinal);
        Assert.Contains(
            "int SetTip(IAvnControl? target, string? value);",
            statics,
            StringComparison.Ordinal);
        Assert.Contains(
            "value = global::Avalonia.Host.Com.AvnToolTipTip.ToAbi(result);",
            statics,
            StringComparison.Ordinal);
        Assert.Contains(
            "global::Avalonia.Controls.ToolTip.SetTip(control, global::Avalonia.Host.Com.AvnToolTipTip.FromAbi(value));",
            statics,
            StringComparison.Ordinal);
        // The scalar members are untouched by the string plumbing.
        Assert.Contains("int SetShowDelay(IAvnControl? target, int value);", statics, StringComparison.Ordinal);
        Assert.Contains("int GetPlacement(IAvnControl? target, out int value);", statics, StringComparison.Ordinal);
        // A scalar getter still zero-initialises without a null-forgiving operator.
        Assert.Contains("public int GetIsOpen(IAvnControl? target, out int value)", statics, StringComparison.Ordinal);
        Assert.Contains("        value = default;", statics, StringComparison.Ordinal);

        // The factory hands out the new statics object alongside the existing ones.
        var factory = files["IAvnControlFactory.g.cs"];
        Assert.Contains("int GetToolTipStatics(out IAvnToolTipStatics? value);", factory, StringComparison.Ordinal);
        Assert.Contains("int CreateImage(out IAvnImage? value);", factory, StringComparison.Ordinal);
        Assert.Contains("int CreateTabControl(out IAvnTabControl? value);", factory, StringComparison.Ordinal);
        Assert.Contains("int CreateTreeViewItem(out IAvnTreeViewItem? value);", factory, StringComparison.Ordinal);
        Assert.Contains("typeof(AvnToolTipStatics)", files["ProjectionAotRoots.g.cs"], StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_the_wave_a_tab_and_tree_controls_over_their_real_bases()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var files = ComSourceEmitter.Emit(ir);

        var tabControl = files["IAvnTabControl.g.cs"];
        Assert.Contains(
            "public partial interface IAvnTabControl : IAvnSelectingItemsControl",
            tabControl,
            StringComparison.Ordinal);
        Assert.Contains("int SetTabStripPlacement(int value);", tabControl, StringComparison.Ordinal);

        var tabItem = files["IAvnTabItem.g.cs"];
        Assert.Contains(
            "public partial interface IAvnTabItem : IAvnHeaderedContentControl",
            tabItem,
            StringComparison.Ordinal);
        Assert.Contains("int SetIsSelected(int value);", tabItem, StringComparison.Ordinal);

        // TreeView's subtree commands take a projected TreeViewItem and unwrap it back to the
        // Avalonia object rather than crossing an opaque handle.
        var treeView = files["IAvnTreeView.g.cs"];
        Assert.Contains(
            "public partial interface IAvnTreeView : IAvnItemsControl",
            treeView,
            StringComparison.Ordinal);
        Assert.Contains(
            "int ExpandSubTreeWithTreeViewItem(IAvnTreeViewItem? item);",
            treeView,
            StringComparison.Ordinal);
        Assert.Contains(
            "_value.ExpandSubTree((global::Avalonia.Controls.TreeViewItem)ProjectionRuntime.Unwrap(item)!);",
            treeView,
            StringComparison.Ordinal);

        var treeViewItem = files["IAvnTreeViewItem.g.cs"];
        Assert.Contains(
            "public partial interface IAvnTreeViewItem : IAvnHeaderedItemsControl",
            treeViewItem,
            StringComparison.Ordinal);
        // Level is computed by the control, so the ABI publishes a getter and no setter.
        Assert.Contains("int GetLevel(out int value);", treeViewItem, StringComparison.Ordinal);
        Assert.DoesNotContain("int SetLevel(int value);", treeViewItem, StringComparison.Ordinal);
        Assert.Contains(
            "int AdviseExpanded(IAvnTreeViewItemExpandedHandler? handler",
            treeViewItem,
            StringComparison.Ordinal);
        Assert.Contains(
            "int AdviseCollapsed(IAvnTreeViewItemCollapsedHandler? handler",
            treeViewItem,
            StringComparison.Ordinal);

        var headered = files["IAvnHeaderedItemsControl.g.cs"];
        Assert.Contains(
            "public partial interface IAvnHeaderedItemsControl : IAvnItemsControl",
            headered,
            StringComparison.Ordinal);
        Assert.Contains("int SetHeader(IAvnControl? value);", headered, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_the_wave_b_flyout_trio_over_avalonia_object_rather_than_control()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var files = ComSourceEmitter.Emit(ir);

        // A flyout is not a control, so the projected interface sits directly on
        // IAvnAvaloniaObject and adds no slot to anything that already shipped.
        var flyoutBase = files["IAvnFlyoutBase.g.cs"];
        Assert.Contains(
            "public partial interface IAvnFlyoutBase : IAvnAvaloniaObject",
            flyoutBase,
            StringComparison.Ordinal);
        // ShowAt takes a control and unwraps it back to the Avalonia object; that method, not an
        // attached property, is how a flyout reaches a control in this wave.
        Assert.Contains(
            "int ShowAtWithControl(IAvnControl? placementTarget);",
            flyoutBase,
            StringComparison.Ordinal);
        Assert.Contains(
            "_value.ShowAt((global::Avalonia.Controls.Control)ProjectionRuntime.Unwrap(placementTarget)!);",
            flyoutBase,
            StringComparison.Ordinal);
        Assert.Contains("int GetTarget(out IAvnControl? value);", flyoutBase, StringComparison.Ordinal);
        Assert.DoesNotContain("int SetTarget(", flyoutBase, StringComparison.Ordinal);

        // The sealed overrides on PopupFlyoutBase are inherited, not republished.
        var popupFlyoutBase = files["IAvnPopupFlyoutBase.g.cs"];
        Assert.Contains(
            "public partial interface IAvnPopupFlyoutBase : IAvnFlyoutBase",
            popupFlyoutBase,
            StringComparison.Ordinal);
        Assert.DoesNotContain(
            "int ShowAtWithControl(IAvnControl placementTarget);",
            popupFlyoutBase,
            StringComparison.Ordinal);
        Assert.Contains("int SetPlacement(int value);", popupFlyoutBase, StringComparison.Ordinal);

        // Cancel is written back into the event args after the handler returns, exactly as
        // Control.KeyDown writes Handled back.
        var closingHandler = files["IAvnPopupFlyoutBaseClosingHandler.g.cs"];
        Assert.Contains("int Invoke(ref int Cancel);", closingHandler, StringComparison.Ordinal);
        Assert.Contains("eventArgs.Cancel = cancel != 0;", popupFlyoutBase, StringComparison.Ordinal);

        var flyout = files["IAvnFlyout.g.cs"];
        Assert.Contains(
            "public partial interface IAvnFlyout : IAvnPopupFlyoutBase",
            flyout,
            StringComparison.Ordinal);
        Assert.Contains("int SetContent(IAvnControl? value);", flyout, StringComparison.Ordinal);

        // No statics interface is minted for a flyout: the attached-property pipeline carries
        // scalars and strings only.
        Assert.False(files.ContainsKey("IAvnFlyoutBaseStatics.g.cs"));
        Assert.DoesNotContain("SetAttachedFlyout", files["IAvnControlFactory.g.cs"], StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_the_wave_b_menu_pair_split_view_and_pickers()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var files = ComSourceEmitter.Emit(ir);

        var menuBase = files["IAvnMenuBase.g.cs"];
        Assert.Contains(
            "public partial interface IAvnMenuBase : IAvnSelectingItemsControl",
            menuBase,
            StringComparison.Ordinal);
        Assert.Contains("int Open();", menuBase, StringComparison.Ordinal);
        Assert.Contains("int Close();", menuBase, StringComparison.Ordinal);
        // The managed setter is protected, so the ABI publishes a getter only.
        Assert.Contains("int GetIsOpen(out int value);", menuBase, StringComparison.Ordinal);
        Assert.DoesNotContain("int SetIsOpen(int value);", menuBase, StringComparison.Ordinal);

        // Menu inherits everything and declares nothing, so Open/Close occupy one slot each.
        var menu = files["IAvnMenu.g.cs"];
        Assert.Contains(
            "public partial interface IAvnMenu : IAvnMenuBase\n{\n}",
            menu.Replace("\r\n", "\n"),
            StringComparison.Ordinal);

        var menuItem = files["IAvnMenuItem.g.cs"];
        Assert.Contains(
            "public partial interface IAvnMenuItem : IAvnHeaderedSelectingItemsControl",
            menuItem,
            StringComparison.Ordinal);
        Assert.Contains("int SetIcon(IAvnControl? value);", menuItem, StringComparison.Ordinal);
        Assert.Contains("int SetToggleType(int value);", menuItem, StringComparison.Ordinal);
        Assert.Contains(
            "int AdviseClick(IAvnMenuItemClickHandler? handler",
            menuItem,
            StringComparison.Ordinal);
        // ICommand projects as IAvnCommand since the command wave.
        Assert.Contains("int SetCommand(IAvnCommand? value);", menuItem, StringComparison.Ordinal);

        var splitView = files["IAvnSplitView.g.cs"];
        Assert.Contains(
            "public partial interface IAvnSplitView : IAvnContentControl",
            splitView,
            StringComparison.Ordinal);
        Assert.Contains("int SetPane(IAvnControl? value);", splitView, StringComparison.Ordinal);
        Assert.Contains("int SetPaneBackground(IAvnBrush? value);", splitView, StringComparison.Ordinal);
        Assert.Contains("int SetOpenPaneLength(double value);", splitView, StringComparison.Ordinal);

        // A date crosses as an ISO-8601 string through a host-side converter; neither half of
        // the conversion is left to the generator, and no date struct is minted.
        var datePicker = files["IAvnDatePicker.g.cs"];
        Assert.Contains("int GetSelectedDate(out string? value);", datePicker, StringComparison.Ordinal);
        Assert.Contains(
            "value = global::Avalonia.Host.Com.AvnDateTimeOffset.ToAbi(_value.SelectedDate);",
            datePicker,
            StringComparison.Ordinal);
        Assert.Contains(
            "_value.SelectedDate = global::Avalonia.Host.Com.AvnDateTimeOffset.FromAbi(value);",
            datePicker,
            StringComparison.Ordinal);
        // MinYear has no absent state, so its slot is a non-nullable string on the other converter.
        Assert.Contains("int SetMinYear(string value);", datePicker, StringComparison.Ordinal);
        Assert.Contains(
            "_value.MinYear = global::Avalonia.Host.Com.AvnDateTimeOffsetValue.FromAbi(value)!",
            datePicker,
            StringComparison.Ordinal);
        Assert.DoesNotContain("System.DateTimeOffset.Parse", datePicker, StringComparison.Ordinal);
        Assert.False(files.ContainsKey("IAvnDateTimeOffset.g.cs"));

        var timePicker = files["IAvnTimePicker.g.cs"];
        Assert.Contains("int SetSelectedTime(string? value);", timePicker, StringComparison.Ordinal);
        Assert.Contains(
            "value = global::Avalonia.Host.Com.AvnTimeSpan.ToAbi(_value.SelectedTime);",
            timePicker,
            StringComparison.Ordinal);

        var factory = files["IAvnControlFactory.g.cs"];
        Assert.All(
            new[]
            {
                "int CreateFlyout(out IAvnFlyout? value);",
                "int CreateMenu(out IAvnMenu? value);",
                "int CreateMenuItem(out IAvnMenuItem? value);",
                "int CreateSplitView(out IAvnSplitView? value);",
                "int CreateDatePicker(out IAvnDatePicker? value);",
                "int CreateTimePicker(out IAvnTimePicker? value);",
            },
            expected => Assert.Contains(expected, factory, StringComparison.Ordinal));
        // The abstract bases are reachable by query_interface only.
        Assert.DoesNotContain("CreateFlyoutBase", factory, StringComparison.Ordinal);
        Assert.DoesNotContain("CreateMenuBase", factory, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_wave_c_layout_panels_and_factory_creators()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var files = ComSourceEmitter.Emit(ir);

        var wrap = files["IAvnWrapPanel.g.cs"];
        Assert.Contains(
            "public partial interface IAvnWrapPanel : IAvnPanel",
            wrap,
            StringComparison.Ordinal);
        Assert.Contains("int SetItemSpacing(double value);", wrap, StringComparison.Ordinal);
        Assert.Contains("int SetItemsAlignment(int value);", wrap, StringComparison.Ordinal);

        var viewbox = files["IAvnViewbox.g.cs"];
        Assert.Contains(
            "public partial interface IAvnViewbox : IAvnControl",
            viewbox,
            StringComparison.Ordinal);
        Assert.Contains("int SetChild(IAvnControl? value);", viewbox, StringComparison.Ordinal);

        var splitter = files["IAvnGridSplitter.g.cs"];
        Assert.Contains(
            "public partial interface IAvnGridSplitter : IAvnThumb",
            splitter,
            StringComparison.Ordinal);
        Assert.Contains("int SetResizeDirection(int value);", splitter, StringComparison.Ordinal);
        Assert.DoesNotContain("SetPreviewContent", splitter, StringComparison.Ordinal);

        var relativeStatics = files["IAvnRelativePanelStatics.g.cs"];
        Assert.Contains(
            "int SetAlignLeftWithPanel(IAvnControl? target, int value);",
            relativeStatics,
            StringComparison.Ordinal);
        Assert.DoesNotContain("SetAbove", relativeStatics, StringComparison.Ordinal);

        var factory = files["IAvnControlFactory.g.cs"];
        Assert.All(
            new[]
            {
                "int CreateWrapPanel(out IAvnWrapPanel? value);",
                "int CreateUniformGrid(out IAvnUniformGrid? value);",
                "int CreateRelativePanel(out IAvnRelativePanel? value);",
                "int CreateViewbox(out IAvnViewbox? value);",
                "int CreateFlexPanel(out IAvnFlexPanel? value);",
                "int CreateThumb(out IAvnThumb? value);",
                "int CreateGridSplitter(out IAvnGridSplitter? value);",
            },
            expected => Assert.Contains(expected, factory, StringComparison.Ordinal));
        Assert.Contains(
            "int GetRelativePanelStatics(out IAvnRelativePanelStatics? value);",
            factory,
            StringComparison.Ordinal);
    }

    [Fact]
    public void Checked_in_ir_and_host_sources_match_generator()
    {
        var ir = ClrTypeExtractor.Extract(KernelTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
        var root = FindRepositoryRoot();

        Assert.Equal(
            Normalize(ir.ToJson()),
            Normalize(File.ReadAllText(Path.Combine(root, "rust", "projection.ir.json"))));

        var generatedDirectory = Path.Combine(root, "host", "Generated", "ObjectModel");
        foreach (var (name, source) in ComSourceEmitter.Emit(ir))
            Assert.Equal(Normalize(source), Normalize(File.ReadAllText(Path.Combine(generatedDirectory, name))));

        Assert.Equal(
            Normalize(NativeHeaderEmitter.Emit(ir)),
            Normalize(File.ReadAllText(Path.Combine(
                root,
                "rust",
                "avalonia-sys",
                "include",
                "avalonia-rust-abi.h"))));

        var header = NativeHeaderEmitter.Emit(ir);
        var buttonStart = header.IndexOf("struct IAvnButtonVtbl", StringComparison.Ordinal);
        var buttonEnd = header.IndexOf("struct IAvnButton {", buttonStart, StringComparison.Ordinal);
        var buttonHeader = header[buttonStart..buttonEnd];
        Assert.Contains("get_object_id", buttonHeader, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_BUTTON_ABI_VERSION 19", header, StringComparison.Ordinal);
        Assert.True(
            buttonHeader.IndexOf("get_object_id", StringComparison.Ordinal) <
            buttonHeader.IndexOf("get_classes", StringComparison.Ordinal));
    }

    [Fact]
    public void Checked_in_view_model_sources_match_generator()
    {
        var root = FindRepositoryRoot();
        var ir = ViewModelIr.FromJson(File.ReadAllText(Path.Combine(
            root,
            "rust",
            "avalonia-sample",
            "view-model.ir.json")));
        var adapterDirectory = Path.Combine(
            root,
            "samples",
            "RustViewModelSample.Managed",
            "Generated");
        var registryDirectory = adapterDirectory;

        foreach (var (name, source) in ViewModelSourceEmitter.EmitCSharp(ir))
        {
            var directory = name == "RustViewRegistry.g.cs"
                ? registryDirectory
                : adapterDirectory;
            Assert.Equal(
                Normalize(source),
                Normalize(File.ReadAllText(Path.Combine(directory, name))));
        }
        Assert.Equal(
            Normalize(ViewModelSourceEmitter.EmitRust(ir, externalConsumer: true)),
            Normalize(File.ReadAllText(Path.Combine(
                root,
                "rust",
                "avalonia-sample",
                "src",
                "generated_view_models.rs"))));
        Assert.Equal(
            Normalize(ViewModelSourceEmitter.EmitContract(ir)),
            Normalize(File.ReadAllText(Path.Combine(
                root,
                "rust",
                "avalonia-sample",
                "view-model.contract.md"))));
    }

    private static string FindRepositoryRoot()
    {
        for (var current = new DirectoryInfo(AppContext.BaseDirectory);
             current is not null;
             current = current.Parent)
        {
            var gitPath = Path.Combine(current.FullName, ".git");
            if (Directory.Exists(gitPath) || File.Exists(gitPath))
                return current.FullName;
        }
        throw new DirectoryNotFoundException("Could not locate repository root.");
    }

    private static string Normalize(string value) =>
        value.Replace("\r\n", "\n", StringComparison.Ordinal).TrimEnd();
}
