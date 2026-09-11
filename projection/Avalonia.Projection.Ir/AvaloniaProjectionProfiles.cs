namespace Avalonia.Projection.Ir;

public static class AvaloniaProjectionProfiles
{
    public static ProjectionPolicy ObjectModelKernel { get; } = new()
    {
        // Waves A and B both add brand-new interfaces and widen no existing one, so every
        // previously published interface keeps the IID it last shipped and the new ones publish
        // at version 1. The default is therefore 1 and every older interface is pinned to the
        // version whose flattened vtable it still matches.
        DefaultProjectedTypeAbiVersion = 1,
        AbiVersions = new Dictionary<string, int>(StringComparer.Ordinal)
        {
            // AvaloniaObject projects no members, so its vtable is byte-identical to
            // version 2. Republishing it under a new IID would be a gratuitous break.
            ["Avalonia.Host.Com.IAvnAvaloniaObject"] = 2,
            // StyledElement is unchanged. Control grows ContextMenu/ContextFlyout/IsLoaded/
            // Loaded/Unloaded, so Control and every descendant republish.
            // StyledElement gains ActualThemeVariant in U27, so it and every descendant republish.
            ["Avalonia.Host.Com.IAvnStyledElement"] = 6,
            ["Avalonia.Host.Com.IAvnControl"] = 11,
            ["Avalonia.Host.Com.IAvnDecorator"] = 13,
            // Everything the completeness wave left alone. None of these sits below
            // ContentControl, Button, ToggleButton, ListBox, ComboBox or Grid, so their
            // flattened vtables are byte-identical to version 4.
            ["Avalonia.Host.Com.IAvnBorder"] = 15,
            ["Avalonia.Host.Com.IAvnTableViewColumn"] = 7,
            ["Avalonia.Host.Com.IAvnPanel"] = 14,
            ["Avalonia.Host.Com.IAvnCanvas"] = 14,
            ["Avalonia.Host.Com.IAvnDockPanel"] = 14,
            ["Avalonia.Host.Com.IAvnStackPanel"] = 15,
            // Wave M grew TemplatedControl (fonts/padding) and TextBlock (fonts/wrapping).
            // Nano-COM vtables are flattened, so every interface below TemplatedControl
            // republishes. TextBlock is not under TemplatedControl; SelectableTextBlock is.
            ["Avalonia.Host.Com.IAvnTextBlock"] = 17,
            ["Avalonia.Host.Com.IAvnSelectableTextBlock"] = 14,
            ["Avalonia.Host.Com.IAvnTemplatedControl"] = 15,
            ["Avalonia.Host.Com.IAvnItemsControl"] = 18,
            ["Avalonia.Host.Com.IAvnSelectingItemsControl"] = 18,
            ["Avalonia.Host.Com.IAvnTextBox"] = 21,
            ["Avalonia.Host.Com.IAvnRangeBase"] = 15,
            ["Avalonia.Host.Com.IAvnSlider"] = 15,
            ["Avalonia.Host.Com.IAvnProgressBar"] = 16,
            ["Avalonia.Host.Com.IAvnContentControl"] = 16,
            ["Avalonia.Host.Com.IAvnHeaderedContentControl"] = 16,
            ["Avalonia.Host.Com.IAvnExpander"] = 16,
            ["Avalonia.Host.Com.IAvnButton"] = 19,
            ["Avalonia.Host.Com.IAvnToggleButton"] = 19,
            ["Avalonia.Host.Com.IAvnCheckBox"] = 19,
            ["Avalonia.Host.Com.IAvnRadioButton"] = 19,
            ["Avalonia.Host.Com.IAvnToggleSwitch"] = 19,
            ["Avalonia.Host.Com.IAvnListBox"] = 19,
            ["Avalonia.Host.Com.IAvnComboBox"] = 20,
            ["Avalonia.Host.Com.IAvnListBoxItem"] = 18,
            ["Avalonia.Host.Com.IAvnComboBoxItem"] = 18,
            ["Avalonia.Host.Com.IAvnScrollViewer"] = 18,
            ["Avalonia.Host.Com.IAvnWindow"] = 21,
            ["Avalonia.Host.Com.IAvnGrid"] = 15,
            // Previously version-1 templated types inherit the TemplatedControl bump.
            ["Avalonia.Host.Com.IAvnAutoCompleteBox"] = 19,
            ["Avalonia.Host.Com.IAvnButtonSpinner"] = 12,
            ["Avalonia.Host.Com.IAvnCalendar"] = 15,
            ["Avalonia.Host.Com.IAvnCalendarDatePicker"] = 14,
            ["Avalonia.Host.Com.IAvnCarousel"] = 16,
            ["Avalonia.Host.Com.IAvnCommandBar"] = 15,
            ["Avalonia.Host.Com.IAvnCommand"] = 2,
            ["Avalonia.Host.Com.IAvnCommandBarButton"] = 14,
            ["Avalonia.Host.Com.IAvnCommandBarSeparator"] = 12,
            ["Avalonia.Host.Com.IAvnCommandBarToggleButton"] = 14,
            ["Avalonia.Host.Com.IAvnContextMenu"] = 18,
            ["Avalonia.Host.Com.IAvnPopup"] = 11,
            ["Avalonia.Host.Com.IAvnTrayIcon"] = 3,
            ["Avalonia.Host.Com.IAvnPopupFlyoutBase"] = 6,
            ["Avalonia.Host.Com.IAvnFlyout"] = 4,
            ["Avalonia.Host.Com.IAvnMenuFlyout"] = 5,
            ["Avalonia.Host.Com.IAvnDatePicker"] = 14,
            ["Avalonia.Host.Com.IAvnDropDownButton"] = 14,
            ["Avalonia.Host.Com.IAvnGridSplitter"] = 13,
            ["Avalonia.Host.Com.IAvnGroupBox"] = 12,
            ["Avalonia.Host.Com.IAvnHeaderedItemsControl"] = 15,
            ["Avalonia.Host.Com.IAvnHeaderedSelectingItemsControl"] = 15,
            ["Avalonia.Host.Com.IAvnHyperlinkButton"] = 14,
            ["Avalonia.Host.Com.IAvnIconElement"] = 12,
            ["Avalonia.Host.Com.IAvnLabel"] = 13,
            ["Avalonia.Host.Com.IAvnMaskedTextBox"] = 18,
            ["Avalonia.Host.Com.IAvnMenu"] = 15,
            ["Avalonia.Host.Com.IAvnMenuBase"] = 15,
            ["Avalonia.Host.Com.IAvnMenuItem"] = 18,
            ["Avalonia.Host.Com.IAvnNotificationCard"] = 13,
            ["Avalonia.Host.Com.IAvnNumericUpDown"] = 13,
            ["Avalonia.Host.Com.IAvnPathIcon"] = 12,
            ["Avalonia.Host.Com.IAvnPipsPager"] = 13,
            ["Avalonia.Host.Com.IAvnRefreshContainer"] = 13,
            ["Avalonia.Host.Com.IAvnRepeatButton"] = 14,
            ["Avalonia.Host.Com.IAvnSeparator"] = 12,
            ["Avalonia.Host.Com.IAvnSpinner"] = 12,
            ["Avalonia.Host.Com.IAvnSplitButton"] = 15,
            ["Avalonia.Host.Com.IAvnSplitView"] = 12,
            ["Avalonia.Host.Com.IAvnTabControl"] = 15,
            ["Avalonia.Host.Com.IAvnTabItem"] = 14,
            ["Avalonia.Host.Com.IAvnTableView"] = 16,
            ["Avalonia.Host.Com.IAvnTableViewCell"] = 14,
            ["Avalonia.Host.Com.IAvnTableViewRow"] = 12,
            ["Avalonia.Host.Com.IAvnThumb"] = 13,
            ["Avalonia.Host.Com.IAvnTimePicker"] = 14,
            ["Avalonia.Host.Com.IAvnToggleSplitButton"] = 15,
            ["Avalonia.Host.Com.IAvnToolTip"] = 12,
            ["Avalonia.Host.Com.IAvnTransitioningContentControl"] = 13,
            ["Avalonia.Host.Com.IAvnTreeView"] = 15,
            ["Avalonia.Host.Com.IAvnTreeViewItem"] = 15,
            ["Avalonia.Host.Com.IAvnUserControl"] = 12,
            ["Avalonia.Host.Com.IAvnWindowNotificationManager"] = 13,
            // Wave A's seven new interfaces publish at the default version 1 and nothing they
            // sit under moved, so they need no entry here. Waves B, C and D do the same.
            // The factory is the only interface these waves move: wave A gave it a creator per
            // new control plus GetToolTipStatics (2 → 3), wave B seven more, wave C seven more,
            // and wave D seven more still (RepeatButton, DropDownButton, SplitButton,
            // ToggleSplitButton, HyperlinkButton, ContextMenu, MenuFlyout), so it republishes
            // at version 6, wave E six more (Spinner is abstract) so 7, and wave F two more
            // (Calendar, CalendarDatePicker) so 8, wave G seven more so 9, and wave H
            // eight constructible shapes (Shape is abstract) so 10, wave I five more so 11,
            // and wave J six more so 12, and wave K five constructible (IconElement abstract) so 13.
            ["Avalonia.Host.Com.IAvnImage"] = 11,
            ["Avalonia.Host.Com.IAvnWrapPanel"] = 11,
            ["Avalonia.Host.Com.IAvnUniformGrid"] = 11,
            ["Avalonia.Host.Com.IAvnRelativePanel"] = 11,
            ["Avalonia.Host.Com.IAvnViewbox"] = 11,
            ["Avalonia.Host.Com.IAvnFlexPanel"] = 11,
            ["Avalonia.Host.Com.IAvnShape"] = 11,
            ["Avalonia.Host.Com.IAvnRectangle"] = 11,
            ["Avalonia.Host.Com.IAvnEllipse"] = 11,
            ["Avalonia.Host.Com.IAvnLine"] = 11,
            ["Avalonia.Host.Com.IAvnPath"] = 11,
            ["Avalonia.Host.Com.IAvnPolygon"] = 11,
            ["Avalonia.Host.Com.IAvnPolyline"] = 11,
            ["Avalonia.Host.Com.IAvnArc"] = 11,
            ["Avalonia.Host.Com.IAvnSector"] = 11,
            ["Avalonia.Host.Com.IAvnLayoutTransformControl"] = 11,
            ["Avalonia.Host.Com.IAvnThemeVariantScope"] = 11,
            ["Avalonia.Host.Com.IAvnControlFactory"] = 13,
            // Wave P. Payload-carrying replacements for the previously payload-less
            // handlers: the handler Invoke slots change shape, so each handler mints a
            // new IID at version 2. The advise/unadvise slots on the control interfaces
            // keep their signatures; the controls themselves republish only because
            // Control gained KeyUp/GotFocus/LostFocus.
            ["Avalonia.Host.Com.IAvnSelectingItemsControlSelectionChangedHandler"] = 2,
            ["Avalonia.Host.Com.IAvnTreeViewSelectionChangedHandler"] = 2,
            ["Avalonia.Host.Com.IAvnRangeBaseValueChangedHandler"] = 2,
            // Wave R: the NumericUpDown value-change handler gains the old/new invariant-string
            // decimal payload, so it mints a new IID at version 2.
            ["Avalonia.Host.Com.IAvnNumericUpDownValueChangedHandler"] = 2,
        },
        IncludeTypeNames =
        [
            "Avalonia.AvaloniaObject",
            "Avalonia.StyledElement",
            "Avalonia.Controls.Control",
            "Avalonia.Controls.ContentControl",
            "Avalonia.Controls.Primitives.HeaderedContentControl",
            "Avalonia.Controls.ItemsControl",
            "Avalonia.Controls.Primitives.HeaderedItemsControl",
            "Avalonia.Controls.Primitives.SelectingItemsControl",
            "Avalonia.Controls.Primitives.HeaderedSelectingItemsControl",
            "Avalonia.Controls.Decorator",
            "Avalonia.Controls.Border",
            "Avalonia.Controls.Panel",
            "Avalonia.Controls.Grid",
            "Avalonia.Controls.Canvas",
            "Avalonia.Controls.DockPanel",
            "Avalonia.Controls.Window",
            "Avalonia.Controls.StackPanel",
            "Avalonia.Controls.TextBlock",
            "Avalonia.Controls.Image",
            "Avalonia.Controls.Primitives.TemplatedControl",
            "Avalonia.Controls.Button",
            "Avalonia.Controls.Primitives.ToggleButton",
            "Avalonia.Controls.CheckBox",
            "Avalonia.Controls.RadioButton",
            "Avalonia.Controls.ToggleSwitch",
            "Avalonia.Controls.Expander",
            "Avalonia.Controls.ListBox",
            "Avalonia.Controls.ComboBox",
            "Avalonia.Controls.ListBoxItem",
            "Avalonia.Controls.ComboBoxItem",
            "Avalonia.Controls.TabControl",
            "Avalonia.Controls.TabItem",
            "Avalonia.Controls.TreeView",
            "Avalonia.Controls.TreeViewItem",
            "Avalonia.Controls.ToolTip",
            // Wave B. A flyout is an AvaloniaObject rather than a Control, so IAvnFlyoutBase
            // hangs directly off IAvnAvaloniaObject and nothing existing moves.
            "Avalonia.Controls.Primitives.FlyoutBase",
            "Avalonia.Controls.Primitives.PopupFlyoutBase",
            "Avalonia.Controls.Flyout",
            "Avalonia.Controls.MenuBase",
            "Avalonia.Controls.Menu",
            "Avalonia.Controls.MenuItem",
            "Avalonia.Controls.SplitView",
            "Avalonia.Controls.DatePicker",
            "Avalonia.Controls.TimePicker",
            // Wave C. Remaining layout panels plus the Thumb base GridSplitter needs.
            // RelativePanel's object-valued attached properties (Above/LeftOf/…) stay gaps:
            // a COM-valued attached property has no ABI shape. The Align*WithPanel bools do
            // cross. Flex's attached properties live on the static Flex class, which is not
            // an AvaloniaObject, so Order/Grow/Shrink/Basis/AlignSelf stay gaps too.
            "Avalonia.Controls.WrapPanel",
            "Avalonia.Controls.Primitives.UniformGrid",
            "Avalonia.Controls.RelativePanel",
            "Avalonia.Controls.Viewbox",
            "Avalonia.Controls.FlexPanel",
            "Avalonia.Controls.Primitives.Thumb",
            "Avalonia.Controls.GridSplitter",
            // Wave D. Button family plus context menus. Flyout on SplitButton/Button stays a
            // gap (COM-valued). NavigateUri crosses as a URI string through AvnUri.
            "Avalonia.Controls.RepeatButton",
            "Avalonia.Controls.DropDownButton",
            "Avalonia.Controls.SplitButton",
            "Avalonia.Controls.ToggleSplitButton",
            "Avalonia.Controls.HyperlinkButton",
            "Avalonia.Controls.ContextMenu",
            "Avalonia.Controls.MenuFlyout",
            // Wave E. Remaining input. Spinner is abstract; NumericUpDown decimals cross as
            // invariant strings. PromptChar is a char and stays a gap.
            "Avalonia.Controls.Spinner",
            "Avalonia.Controls.ButtonSpinner",
            "Avalonia.Controls.NumericUpDown",
            "Avalonia.Controls.AutoCompleteBox",
            "Avalonia.Controls.MaskedTextBox",
            "Avalonia.Controls.SelectableTextBlock",
            // Wave F. Calendar days are DateTime, not DateTimeOffset, so they cross as
            // yyyy-MM-dd through AvnCalendarDate rather than the picker "o" form.
            "Avalonia.Controls.Calendar",
            "Avalonia.Controls.CalendarDatePicker",
            // Wave G. Content chrome. PageTransition and LayoutTransform are interfaces
            // with no ABI shape; Label.Target is IInputElement, not a projected control.
            "Avalonia.Controls.Carousel",
            "Avalonia.Controls.TransitioningContentControl",
            "Avalonia.Controls.Label",
            "Avalonia.Controls.Separator",
            "Avalonia.Controls.GroupBox",
            "Avalonia.Controls.UserControl",
            "Avalonia.Controls.LayoutTransformControl",
            // Wave H. Shapes. Fill/Stroke are brushes. Path.Data is Geometry with Parse/ToString.
            // Points collections stay gaps. Shape is abstract.
            "Avalonia.Controls.Shapes.Shape",
            "Avalonia.Controls.Shapes.Rectangle",
            "Avalonia.Controls.Shapes.Ellipse",
            "Avalonia.Controls.Shapes.Line",
            "Avalonia.Controls.Shapes.Path",
            "Avalonia.Controls.Shapes.Polygon",
            "Avalonia.Controls.Shapes.Polyline",
            "Avalonia.Controls.Shapes.Arc",
            "Avalonia.Controls.Shapes.Sector",
            // Wave I. Overlay and notifications. TrayIcon is an AvaloniaObject, not a Control.
            // NativeMenu, WindowIcon and ICommand stay gaps.
            "Avalonia.Controls.Primitives.Popup",
            "Avalonia.Controls.TrayIcon",
            "Avalonia.Controls.Notifications.WindowNotificationManager",
            "Avalonia.Controls.Notifications.NotificationCard",
            "Avalonia.Controls.RefreshContainer",
            // Wave J. CommandBar family, PipsPager, ThemeVariantScope. Command lists, Icon
            // and ThemeVariant stay gaps.
            "Avalonia.Controls.CommandBar",
            "Avalonia.Controls.CommandBarButton",
            "Avalonia.Controls.CommandBarToggleButton",
            "Avalonia.Controls.CommandBarSeparator",
            "Avalonia.Controls.PipsPager",
            "Avalonia.Controls.ThemeVariantScope",
            // Wave K. Icons and TableView. Inlines stay a gap. Column Width is a GridLength
            // string. Columns is a projected collection of TableViewColumn.
            "Avalonia.Controls.IconElement",
            "Avalonia.Controls.PathIcon",
            "Avalonia.Controls.TableView",
            "Avalonia.Controls.TableViewColumn",
            "Avalonia.Controls.TableViewRow",
            "Avalonia.Controls.TableViewCell",
            "Avalonia.Controls.TextBox",
            "Avalonia.Controls.ScrollViewer",
            "Avalonia.Controls.Primitives.RangeBase",
            "Avalonia.Controls.Slider",
            "Avalonia.Controls.ProgressBar",
        ],
        IncludeMembers = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
        {
            ["Avalonia.AvaloniaObject"] = [],
            ["Avalonia.StyledElement"] =
            [
                "Classes", "Name", "DataContextChanged", "DataContext", "IsInitialized",
                "Parent", "TemplatedParent", "Initialized", "ActualThemeVariantChanged",
                "ResourcesChanged", "AttachedToLogicalTree", "DetachedFromLogicalTree",
                "ActualThemeVariant",
            ],
            ["Avalonia.Controls.Control"] =
            [
                "Width", "Height", "MinWidth", "MinHeight", "MaxWidth", "MaxHeight",
                "Margin", "HorizontalAlignment", "VerticalAlignment", "IsVisible",
                "Opacity", "IsEnabled", "KeyDown", "KeyUp", "GotFocus", "LostFocus",
                "PointerEntered", "PointerExited", "Tapped", "DoubleTapped",
                "PointerWheelChanged", "Focus", "Focusable",
                "ContextMenu", "ContextFlyout", "IsLoaded", "Loaded", "Unloaded",
                "SizeChanged", "Tag",
            ],
            ["Avalonia.Controls.ContentControl"] =
                ["Content", "HorizontalContentAlignment", "VerticalContentAlignment", "ContentTemplate"],
            ["Avalonia.Controls.Primitives.HeaderedContentControl"] = ["Header", "HeaderTemplate"],
            ["Avalonia.Controls.ItemsControl"] = ["Items", "ItemCount", "ScrollIntoView", "ContainerFromIndex", "IndexFromContainer", "ItemsSource", "ContainerFromItem", "ItemFromContainer", "ItemTemplate"],
            ["Avalonia.Controls.Primitives.HeaderedItemsControl"] = ["Header", "HeaderTemplate"],
            ["Avalonia.Controls.Primitives.SelectingItemsControl"] =
            [
                "SelectedIndex", "SelectionChanged", "SelectedItem", "SelectedValue",
                "AutoScrollToSelectedItem", "IsTextSearchEnabled", "WrapSelection", "SelectedItems",
            ],
            ["Avalonia.Controls.Primitives.HeaderedSelectingItemsControl"] = ["Header", "HeaderTemplate"],
            ["Avalonia.Controls.Decorator"] = ["Child", "Padding"],
            ["Avalonia.Controls.Border"] =
                ["Background", "BorderBrush", "BorderThickness", "CornerRadius", "BackgroundSizing",
                 "ClipToBoundsRadius", "BoxShadow"],
            ["Avalonia.Controls.Panel"] = ["Background", "Children"],
            ["Avalonia.Controls.Grid"] =
                ["ShowGridLines", "RowSpacing", "ColumnSpacing", "ColumnDefinitions", "RowDefinitions"],
            ["Avalonia.Controls.Canvas"] = [],
            ["Avalonia.Controls.DockPanel"] = ["LastChildFill", "HorizontalSpacing", "VerticalSpacing"],
            ["Avalonia.Controls.Window"] =
            [
                "Title", "CanResize", "WindowState", "Show", "Close", "Hide",
                "SizeToContent", "ShowActivated", "ShowInTaskbar", "CanMinimize",
                "CanMaximize", "WindowStartupLocation", "WindowDecorations",
                "ClosingBehavior", "ExtendClientAreaToDecorationsHint",
                "ExtendClientAreaTitleBarHeightHint", "IsExtendedIntoWindowDecorations",
                "WindowDecorationMargin", "OffScreenMargin", "IsDialog", "Closing", "Position",
                "Icon",
            ],
            ["Avalonia.Controls.StackPanel"] =
                ["Orientation", "Spacing", "AreHorizontalSnapPointsRegular", "AreVerticalSnapPointsRegular"],
            ["Avalonia.Controls.TextBlock"] =
            [
                "Text", "FontSize", "FontWeight", "FontFamily", "FontStyle", "FontStretch",
                "Foreground", "Background", "Padding", "TextAlignment", "LetterSpacing",
                "LineSpacing", "LineHeight", "BaselineOffset", "MaxLines", "TextWrapping",
                "TextTrimming", "FontFeatures",
            ],
            ["Avalonia.Controls.Image"] =
                ["Source", "Stretch", "StretchDirection", "BlendMode"],
            ["Avalonia.Controls.Button"] =
                ["ClickMode", "IsDefault", "IsCancel", "IsPressed", "Click", "Flyout", "Command", "CommandParameter", "HotKey"],
            ["Avalonia.Controls.Primitives.ToggleButton"] =
                ["IsChecked", "IsThreeState", "IsCheckedChanged"],
            ["Avalonia.Controls.CheckBox"] = [],
            ["Avalonia.Controls.RadioButton"] = ["GroupName"],
            ["Avalonia.Controls.ToggleSwitch"] = ["OnContent", "OffContent", "OnContentTemplate", "OffContentTemplate"],
            ["Avalonia.Controls.Expander"] =
                ["ExpandDirection", "IsExpanded", "Expanded", "Collapsed", "Expanding", "Collapsing"],
            ["Avalonia.Controls.ListBox"] = ["SelectionMode", "SelectAll", "UnselectAll"],
            ["Avalonia.Controls.ComboBox"] =
            [
                "PlaceholderText", "IsDropDownOpen", "IsEditable", "MaxDropDownHeight", "SelectionBoxItemTemplate",
                "SelectionBoxItem",
                "Text", "PlaceholderForeground", "Clear", "DropDownOpened", "DropDownClosed",
            ],
            ["Avalonia.Controls.ListBoxItem"] = ["IsSelected"],
            ["Avalonia.Controls.ComboBoxItem"] = [],
            // TabControl inherits Items and SelectedIndex from SelectingItemsControl, so it only
            // publishes what it declares itself.
            ["Avalonia.Controls.TabControl"] =
                ["TabStripPlacement", "HorizontalContentAlignment", "VerticalContentAlignment", "SelectedContent", "ContentTemplate", "IndicatorTemplate", "SelectedContentTemplate"],
            // TabItem.TabStripPlacement crosses as a nullable Dock through the AvnDock
            // host converter (null string = unset placement).
            ["Avalonia.Controls.TabItem"] =
                ["IsSelected", "Icon", "IconTemplate", "IndicatorTemplate", "TabStripPlacement"],
            // TreeView derives from ItemsControl rather than SelectingItemsControl, so it carries
            // Items but no SelectedIndex. SelectedItem/SelectedItems are object/IList and stay in
            // the gap report.
            ["Avalonia.Controls.TreeView"] =
            [
                "AutoScrollToSelectedItem", "SelectionMode", "SelectAll", "UnselectAll",
                "ExpandSubTree", "CollapseSubTree", "SelectionChanged", "SelectedItem",
                "TreeContainerFromItem", "TreeItemFromContainer", "SelectedItems",
            ],
            ["Avalonia.Controls.TreeViewItem"] =
                ["IsExpanded", "IsSelected", "Level", "Expanded", "Collapsed"],
            // ToolTip is projected for its attached properties; as a control it adds nothing
            // over ContentControl.
            ["Avalonia.Controls.ToolTip"] = [],
            // A flyout is shown imperatively: there is no attached-property pipeline for a
            // COM-valued property, so ShowAt/Hide are how a flyout reaches a control.
            ["Avalonia.Controls.Primitives.FlyoutBase"] =
                ["IsOpen", "Target", "ShowAt", "Hide", "Opened", "Closed"],
            // PopupFlyoutBase re-declares ShowAt/Hide as sealed overrides; they are inherited
            // from IAvnFlyoutBase rather than published twice. Placement flags cross as I32.
            ["Avalonia.Controls.Primitives.PopupFlyoutBase"] =
            [
                "Placement", "ShowMode", "HorizontalOffset", "VerticalOffset",
                "OverlayDismissEventPassThrough", "OverlayInputPassThroughElement", "ShowAt", "Opening", "Closing",
                "Popup",
                "PlacementAnchor", "PlacementGravity", "PlacementConstraintAdjustment", "CustomPopupPlacementCallback"
            ],
            ["Avalonia.Controls.Flyout"] = ["Content", "ContentTemplate"],
            // Menu is imperative, unlike the view-model NativeMenu: MenuBase owns the open
            // state and Menu inherits it, declaring nothing of its own.
            ["Avalonia.Controls.MenuBase"] = ["IsOpen", "Open", "Close", "Opened", "Closed"],
            ["Avalonia.Controls.Menu"] = [],
            // Command and CommandParameter are an ICommand and an object; HotKey and
            // InputGesture are KeyGestures that cross through KeyGesture's own
            // Parse/ToString round-trip.
            ["Avalonia.Controls.MenuItem"] =
            [
                "Command",
                "CommandParameter",
                "Icon", "IsSelected", "IsSubMenuOpen", "StaysOpenOnClick", "ToggleType",
                "IsChecked", "GroupName", "Click", "SubmenuOpened",
                "HasSubMenu", "IsTopLevel", "Open", "Close", "HotKey", "InputGesture",
            ],
            ["Avalonia.Controls.SplitView"] =
            [
                "IsPaneOpen", "DisplayMode", "PanePlacement", "OpenPaneLength",
                "CompactPaneLength", "Pane", "PaneBackground", "UseLightDismissOverlayMode",
                "PaneOpened", "PaneClosed", "PaneOpening", "PaneClosing", "PaneTemplate",
            ],
            // U28 crosses SelectedDateChanged: the old/new DateTimeOffset? pair rides the
            // optional DateTime ABI wrapper as event fields.
            ["Avalonia.Controls.DatePicker"] =
            [
                "SelectedDate", "MinYear", "MaxYear", "DayVisible", "MonthVisible",
                "YearVisible", "DayFormat", "MonthFormat", "YearFormat", "Clear",
                "VerticalContentAlignment", "SelectedDateChanged",
            ],
            // U28 crosses SelectedTimeChanged: the old/new TimeSpan? pair rides the
            // optional TimeSpan ABI wrapper as event fields.
            ["Avalonia.Controls.TimePicker"] =
            [
                "SelectedTime", "MinuteIncrement", "SecondIncrement", "ClockIdentifier",
                "UseSeconds", "Clear", "VerticalContentAlignment", "SelectedTimeChanged",
            ],
            ["Avalonia.Controls.WrapPanel"] =
            [
                "Orientation", "ItemWidth", "ItemHeight", "ItemSpacing", "LineSpacing",
                "ItemsAlignment",
            ],
            ["Avalonia.Controls.Primitives.UniformGrid"] =
                ["Rows", "Columns", "FirstColumn", "RowSpacing", "ColumnSpacing"],
            ["Avalonia.Controls.RelativePanel"] = [],
            ["Avalonia.Controls.Viewbox"] = ["Child", "Stretch", "StretchDirection"],
            ["Avalonia.Controls.FlexPanel"] =
            [
                "Direction", "JustifyContent", "AlignItems", "AlignContent", "Wrap",
                "ColumnSpacing", "RowSpacing",
            ],
            ["Avalonia.Controls.Primitives.Thumb"] =
                ["DragStarted", "DragDelta", "DragCompleted"],
            ["Avalonia.Controls.GridSplitter"] =
            [
                "ResizeDirection", "ResizeBehavior", "ShowsPreview", "KeyboardIncrement",
                "DragIncrement",
            ],
            ["Avalonia.Controls.RepeatButton"] = ["Interval", "Delay"],
            ["Avalonia.Controls.DropDownButton"] = [],
            ["Avalonia.Controls.SplitButton"] = ["Click", "Flyout", "Command", "CommandParameter", "HotKey"],
            ["Avalonia.Controls.ToggleSplitButton"] = ["IsChecked", "IsCheckedChanged"],
            ["Avalonia.Controls.HyperlinkButton"] = ["IsVisited", "NavigateUri"],
            ["Avalonia.Controls.ContextMenu"] =
            [
                "HorizontalOffset", "VerticalOffset", "Placement", "WindowManagerAddShadowHint",
                "PlacementTarget", "CustomPopupPlacementCallback",
                "PlacementAnchor", "PlacementGravity", "PlacementConstraintAdjustment",
                "PlacementRect",
                "Open", "Opening", "Closing",
            ],
            ["Avalonia.Controls.MenuFlyout"] = ["Items", "ItemsSource", "ItemTemplate"],
            ["Avalonia.Controls.Spinner"] = ["ValidSpinDirection", "Spin"],
            ["Avalonia.Controls.ButtonSpinner"] =
                ["AllowSpin", "ShowButtonSpinner", "ButtonSpinnerLocation"],
            ["Avalonia.Controls.NumericUpDown"] =
            [
                "Value", "Minimum", "Maximum", "Increment", "Text", "PlaceholderText",
                "IsReadOnly", "ClipValueToMinMax", "AllowSpin", "ShowButtonSpinner",
                "ButtonSpinnerLocation", "FormatString",
                "PlaceholderForeground", "HorizontalContentAlignment", "VerticalContentAlignment",
                "TextAlignment", "InnerLeftContent", "InnerRightContent", "ValueChanged",
                "Spinned",
            ],
            ["Avalonia.Controls.AutoCompleteBox"] =
            [
                "Text", "PlaceholderText", "MinimumPrefixLength", "MaxDropDownHeight",
                "IsDropDownOpen", "FilterMode", "IsTextCompletionEnabled",
                "CaretIndex", "ClearSelectionOnLostFocus", "SearchText", "MaxLength",
                "PlaceholderForeground", "InnerLeftContent", "InnerRightContent",
                "PopulateComplete", "DropDownOpened", "DropDownClosed",
                "MinimumPopulateDelay", "TextChanged", "ItemsSource",
                "Populating", "DropDownOpening", "DropDownClosing", "SelectedItem", "ItemTemplate",
                "ItemFilter", "TextFilter", "ItemSelector", "TextSelector", "AsyncPopulator",
                "SelectionChanged", "Populated",
            ],
            ["Avalonia.Controls.MaskedTextBox"] =
                ["Mask", "AsciiOnly", "HidePromptOnLeave", "ResetOnPrompt", "ResetOnSpace",
                 "MaskCompleted", "MaskFull", "PromptChar"],
            ["Avalonia.Controls.SelectableTextBlock"] =
            [
                "SelectionStart", "SelectionEnd", "SelectedText", "CanCopy", "Copy",
                "SelectionBrush", "SelectionForegroundBrush", "SelectAll", "ClearSelection",
                "CopyingToClipboard",
            ],
            ["Avalonia.Controls.Calendar"] =
            [
                "SelectedDate", "DisplayDate", "DisplayDateStart", "DisplayDateEnd",
                "DisplayMode", "SelectionMode", "IsTodayHighlighted", "FirstDayOfWeek",
                "HeaderBackground", "IsWeekNumberVisible", "WeekNumberRule",
                "AllowTapRangeSelection", "DisplayModeChanged", "DisplayDateChanged",
                "SelectedDates", "BlackoutDates", "SelectedDatesChanged",
            ],
            ["Avalonia.Controls.CalendarDatePicker"] =
            [
                "SelectedDate", "DisplayDate", "DisplayDateStart", "DisplayDateEnd",
                "IsDropDownOpen", "IsTodayHighlighted", "SelectedDateFormat",
                "CustomDateFormatString", "Text", "PlaceholderText", "IsWeekNumberVisible",
                "FirstDayOfWeek", "UseFloatingPlaceholder", "PlaceholderForeground",
                "DateValidationError",
                "HorizontalContentAlignment", "VerticalContentAlignment", "WeekNumberRule",
                "CalendarOpened", "CalendarClosed", "Clear", "BlackoutDates", "SelectedDateChanged",
            ],
            ["Avalonia.Controls.Carousel"] =
                ["IsSwipeEnabled", "ViewportFraction", "IsSwiping", "Next", "Previous"],
            ["Avalonia.Controls.TransitioningContentControl"] = ["IsTransitionReversed", "TransitionCompleted"],
            ["Avalonia.Controls.Label"] = ["Target"],
            ["Avalonia.Controls.Separator"] = [],
            ["Avalonia.Controls.GroupBox"] = [],
            ["Avalonia.Controls.UserControl"] = [],
            ["Avalonia.Controls.LayoutTransformControl"] = ["UseRenderTransform"],
            ["Avalonia.Controls.Shapes.Shape"] =
            [
                "Fill", "Stroke", "StrokeThickness", "Stretch", "StrokeDashOffset",
                "StrokeLineCap", "StrokeJoin", "StrokeMiterLimit", "StrokeDashArray",
            ],
            ["Avalonia.Controls.Shapes.Rectangle"] = ["RadiusX", "RadiusY"],
            ["Avalonia.Controls.Shapes.Ellipse"] = [],
            ["Avalonia.Controls.Shapes.Line"] = ["StartPoint", "EndPoint"],
            ["Avalonia.Controls.Shapes.Path"] = ["Data"],
            ["Avalonia.Controls.Shapes.Polygon"] = ["FillRule", "Points"],
            ["Avalonia.Controls.Shapes.Polyline"] = ["FillRule", "Points"],
            ["Avalonia.Controls.Shapes.Arc"] = ["StartAngle", "SweepAngle"],
            ["Avalonia.Controls.Shapes.Sector"] = ["StartAngle", "SweepAngle"],
            ["Avalonia.Controls.Primitives.Popup"] =
            [
                "Child", "IsOpen", "Placement", "HorizontalOffset", "VerticalOffset",
                "IsLightDismissEnabled", "Topmost", "WindowManagerAddShadowHint", "OverlayInputPassThroughElement",
                "OverlayDismissEventPassThrough",
                "InheritsTransform", "PlacementTarget", "TakesFocusFromNativeControl",
                "ShouldUseOverlayLayer", "IsUsingOverlayLayer", "IsPointerOverPopup",
                "Opened", "Closed", "Open", "Close",
                "PlacementAnchor", "PlacementGravity", "PlacementConstraintAdjustment", "CustomPopupPlacementCallback",
                "PlacementRect",
            ],
            ["Avalonia.Controls.TrayIcon"] = ["ToolTipText", "IsVisible", "Command", "CommandParameter", "Icon", "Clicked"],
            ["Avalonia.Controls.Notifications.WindowNotificationManager"] = ["Position", "MaxItems", "Show", "Close", "CloseAll"],
            ["Avalonia.Controls.Notifications.NotificationCard"] =
                ["IsClosed", "NotificationType", "IsClosing", "Close", "NotificationClosed"],
            ["Avalonia.Controls.RefreshContainer"] =
                ["PullDirection", "IsMouseEnabled", "RequestRefresh", "RefreshRequested"],
            ["Avalonia.Controls.CommandBar"] =
            [
                "Content", "DefaultLabelPosition", "IsDynamicOverflowEnabled",
                "OverflowButtonVisibility", "IsOpen", "IsSticky",
                "ItemWidthBottom", "ItemWidthRight", "ItemWidthCollapsed",
                "HasSecondaryCommands", "IsOverflowButtonVisible",
                "PrimaryCommands", "SecondaryCommands", "VisiblePrimaryCommands",
                "OverflowItems",
                "Opening", "Opened", "Closing", "Closed",
            ],
            ["Avalonia.Controls.CommandBarButton"] =
                ["Label", "IsCompact", "DynamicOverflowOrder", "LabelPosition", "IsInOverflow", "Icon"],
            ["Avalonia.Controls.CommandBarToggleButton"] =
                ["Label", "IsCompact", "DynamicOverflowOrder", "LabelPosition", "IsInOverflow", "Icon"],
            ["Avalonia.Controls.CommandBarSeparator"] = ["IsCompact", "IsInOverflow"],
            ["Avalonia.Controls.PipsPager"] =
            [
                "MaxVisiblePips", "NumberOfPages", "SelectedPageIndex", "Orientation",
                "IsNextButtonVisible", "IsPreviousButtonVisible", "SelectedIndexChanged",
            ],
            ["Avalonia.Controls.ThemeVariantScope"] = ["RequestedThemeVariant"],
            ["Avalonia.Controls.IconElement"] = [],
            ["Avalonia.Controls.PathIcon"] = ["Data"],
            ["Avalonia.Controls.TableView"] = ["CanUserResizeColumns", "Columns"],
            ["Avalonia.Controls.TableViewColumn"] =
            [
                "Header", "HeaderTemplate", "Width", "MinWidth", "MaxWidth", "IsVisible",
                "HorizontalContentAlignment", "CanUserResize",
                "ActualWidth", "CanUserEffectivelyResize",
            ],
            ["Avalonia.Controls.TableViewRow"] = [],
            ["Avalonia.Controls.TableViewCell"] = [],
            ["Avalonia.Controls.Primitives.TemplatedControl"] =
            [
                "Background", "BorderBrush", "BorderThickness", "CornerRadius", "FontSize",
                "FontFamily", "FontStyle", "FontWeight", "FontStretch", "LetterSpacing",
                "Foreground", "Padding", "FontFeatures", "BackgroundSizing",
            ],
            ["Avalonia.Controls.TextBox"] =
            [
                "Text", "PlaceholderText", "AcceptsReturn", "AcceptsTab", "IsReadOnly",
                "CaretIndex", "SelectionStart", "SelectionEnd", "MaxLength", "MaxLines",
                "MinLines", "LineHeight", "RevealPassword", "TextWrapping", "NewLine",
                "IsUndoEnabled", "UndoLimit", "CanUndo", "CanRedo", "CanCut", "CanCopy",
                "CanPaste", "Clear", "Cut", "Copy", "Paste", "Undo", "Redo", "TextChanged",
                "SelectedText", "HorizontalContentAlignment", "VerticalContentAlignment",
                "TextAlignment", "SelectionBrush", "SelectionForegroundBrush", "CaretBrush",
                "IsInactiveSelectionHighlightEnabled", "ClearSelectionOnLostFocus",
                "UseFloatingPlaceholder", "PlaceholderForeground", "InnerLeftContent",
                "InnerRightContent", "SelectAll", "ClearSelection", "ScrollToLine", "GetLineCount",
                "PasswordChar", "CaretBlinkInterval", "TextChanging",
                "CopyingToClipboard", "CuttingToClipboard", "PastingFromClipboard",
            ],
            ["Avalonia.Controls.ScrollViewer"] =
            [
                "BringIntoViewOnFocusChange", "HorizontalScrollBarVisibility",
                "VerticalScrollBarVisibility", "AllowAutoHide", "IsScrollChainingEnabled",
                "IsScrollInertiaEnabled", "IsDeferredScrollingEnabled", "IsExpanded",
                "LineUp", "LineDown", "LineLeft", "LineRight", "PageUp", "PageDown",
                "PageLeft", "PageRight", "ScrollToHome", "ScrollToEnd", "ScrollChanged",
                "Extent", "Offset", "Viewport", "LargeChange", "SmallChange",
                "HorizontalSnapPointsType", "VerticalSnapPointsType",
                "HorizontalSnapPointsAlignment", "VerticalSnapPointsAlignment",
                "ScrollBarMaximum", "CurrentAnchor",
                "RegisterAnchorCandidate", "UnregisterAnchorCandidate",
            ],
            ["Avalonia.Controls.Primitives.RangeBase"] =
                ["Minimum", "Maximum", "Value", "SmallChange", "LargeChange", "ValueChanged"],
            ["Avalonia.Controls.Slider"] =
                ["Orientation", "IsDirectionReversed", "IsSnapToTickEnabled", "TickFrequency", "TickPlacement", "Ticks"],
            ["Avalonia.Controls.ProgressBar"] =
                ["IsIndeterminate", "ShowProgressText", "ProgressTextFormat", "Orientation", "Percentage"],
        },
        MemberOverrides = new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal)
        {
            ["Avalonia.Controls.ContentControl.Content"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Panel.Children"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                IsNullable = false,
            },
            // CommandParameter is object?: the ABI carries a tagged scalar that
            // covers the values a command parameter can hold without a full
            // object-model bridge — null, text, i32, f64, bool.
            ["Avalonia.Controls.Button.CommandParameter"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuItem.CommandParameter"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.SplitButton.CommandParameter"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.ToggleSplitButton.CommandParameter"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.TrayIcon.CommandParameter"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.ItemsControl.Items"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnItemList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                IsNullable = false,
            },
            // ItemsSource is IEnumerable?: the adapter materializes on first
            // mutation, and assigning the list back persists it into the control.
            ["Avalonia.Controls.ItemsControl.ItemsSource"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnVariantList",
                ElementKind = MarshallingKind.Variant,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnObjectList",
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuFlyout.ItemsSource"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnVariantList",
                ElementKind = MarshallingKind.Variant,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnObjectList",
                IsNullable = true,
            },
            ["Avalonia.Controls.AutoCompleteBox.ItemsSource"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnVariantList",
                ElementKind = MarshallingKind.Variant,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnObjectList",
                IsNullable = true,
            },
            // SelectedItem/SelectedValue are object?: the tagged scalar carries
            // the values an item can hold without a typed item ABI.
            ["Avalonia.Controls.Primitives.SelectingItemsControl.SelectedItem"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.SelectingItemsControl.SelectedValue"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.Window.Icon"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnWindowIcon",
                IsNullable = true,
            },
            ["Avalonia.Controls.Shapes.Polygon.Points"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnPointList",
                IsNullable = false,
            },
            ["Avalonia.Controls.Shapes.Polyline.Points"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnPointList",
                IsNullable = false,
            },
            ["Avalonia.Controls.Shapes.Shape.StrokeDashArray"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDoubleList",
                IsNullable = true,
            },
            ["Avalonia.Controls.TextBlock.FontFeatures"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnFontFeatures",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.TemplatedControl.FontFeatures"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnFontFeatures",
                IsNullable = true,
            },
            ["Avalonia.Controls.Calendar.SelectedDates"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnDateTimeList",
                ElementKind = MarshallingKind.DateTimeI64,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnDateList",
                IsNullable = false,
            },
            ["Avalonia.Controls.Calendar.BlackoutDates"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnDateTimeList",
                ElementKind = MarshallingKind.DateTimeI64,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnDateList",
                IsNullable = false,
            },
            ["Avalonia.Controls.CalendarDatePicker.BlackoutDates"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnDateTimeList",
                ElementKind = MarshallingKind.DateTimeI64,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnDateList",
                IsNullable = false,
            },
            ["Avalonia.Controls.TrayIcon.Icon"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnWindowIcon",
                IsNullable = true,
            },
            ["Avalonia.Controls.Slider.Ticks"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDoubleList",
                IsNullable = true,
            },
            // U36: the async populator crosses as IAvnAsyncPopulator — BeginPopulate
            // launches and the CCW reports through IAvnAsyncPopulatorCompletion.
            ["Avalonia.Controls.AutoCompleteBox.AsyncPopulator"] = new()
            {
                Kind = MarshallingKind.AsyncPopulator,
                InterfaceName = "Avalonia.Host.Com.IAvnAsyncPopulator",
                IsNullable = true,
            },
            ["Avalonia.Controls.AutoCompleteBox.SelectedItem"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.TreeView.SelectedItem"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.SelectingItemsControl.SelectedItems"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnSelectedVariantList",
                ElementKind = MarshallingKind.Variant,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnObjectList",
                IsNullable = true,
            },
            ["Avalonia.Controls.TreeView.SelectedItems"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnSelectedVariantList",
                ElementKind = MarshallingKind.Variant,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnObjectList",
                IsNullable = true,
            },
            ["Avalonia.StyledElement.Classes"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnStringList",
                ElementKind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            // Grid's track definitions cross as the same comma-separated length list that
            // ColumnDefinitions/RowDefinitions already parse and print, not as a projected
            // collection of definition objects. The host wrapper converts with the type's own
            // Parse/ToString, so nothing here needs an interface of its own.
            ["Avalonia.Controls.Primitives.TemplatedControl.FontFamily"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            ["Avalonia.Controls.TextBlock.FontFamily"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            ["Avalonia.Controls.TextBlock.TextTrimming"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnTextTrimming",
                IsNullable = false,
            },
            ["Avalonia.Controls.Label.Target"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Control.ContextMenu"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnContextMenu",
                IsNullable = true,
            },
            ["Avalonia.Controls.Control.ContextFlyout"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnFlyoutBase",
                IsNullable = true,
            },
            ["Avalonia.Controls.TextBox.InnerLeftContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.TextBox.InnerRightContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.AutoCompleteBox.InnerLeftContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.AutoCompleteBox.InnerRightContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.NumericUpDown.InnerLeftContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.NumericUpDown.InnerRightContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Grid.ColumnDefinitions"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            ["Avalonia.Controls.Grid.RowDefinitions"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            ["Avalonia.Controls.Decorator.Child"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.HeaderedContentControl.Header"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.HeaderedItemsControl.Header"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.HeaderedSelectingItemsControl.Header"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Flyout.Content"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuItem.Icon"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Button.Flyout"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnFlyoutBase",
                IsNullable = true,
            },
            ["Avalonia.Controls.SplitButton.Flyout"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnFlyoutBase",
                IsNullable = true,
            },
            ["Avalonia.Controls.SplitView.Pane"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Viewbox.Child"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.HyperlinkButton.NavigateUri"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnUri",
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuFlyout.Items"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnItemList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                IsNullable = false,
            },
            // U30: the command-bar command lists are IList<ICommandBarElement>, whose
            // elements are projected controls; the host adapter is a live write-through
            // view so mutations persist into the bar.
            ["Avalonia.Controls.CommandBar.PrimaryCommands"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnControlList",
                IsNullable = false,
            },
            ["Avalonia.Controls.CommandBar.SecondaryCommands"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnControlList",
                IsNullable = false,
            },
            // The read-only views cross through the same live adapter: reads observe the
            // bar's realized collections, and the mutating slots fail E_POINTER because
            // the CLR collection is read-only.
            ["Avalonia.Controls.CommandBar.VisiblePrimaryCommands"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnControlList",
                IsNullable = false,
            },
            // U33: the table-view column list crosses through the same live control-list
            // adapter as the command-bar lists (AvaloniaList<TableViewColumn> is an IList
            // of projected controls).
            ["Avalonia.Controls.TableView.Columns"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnControlList",
                IsNullable = true,
            },
            ["Avalonia.Controls.CommandBar.OverflowItems"] = new()
            {
                Kind = MarshallingKind.ComCollection,
                InterfaceName = "Avalonia.Host.Com.IAvnControlList",
                ElementInterfaceName = "Avalonia.Host.Com.IAvnControl",
                ElementKind = MarshallingKind.ComInterface,
                HostImplementationTypeName = "Avalonia.Host.Com.AvnControlList",
                IsNullable = false,
            },
            // U30: the untyped user payload rides the Variant kind.
            ["Avalonia.Controls.Control.Tag"] = new()
            {
                Kind = MarshallingKind.Variant,
                IsNullable = true,
            },
            // U30: the pass-through element is an IInputElement; only projected controls
            // cross, the same shape Label.Target already uses.
            ["Avalonia.Controls.Primitives.Popup.OverlayInputPassThroughElement"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.PopupFlyoutBase.OverlayInputPassThroughElement"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.NumericUpDown.Value"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDecimal",
                IsNullable = true,
            },
            ["Avalonia.Controls.NumericUpDown.Minimum"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDecimalValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.NumericUpDown.Maximum"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDecimalValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.NumericUpDown.Increment"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDecimalValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.Calendar.SelectedDate"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.Calendar.DisplayDate"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDateValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.Calendar.DisplayDateStart"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.Calendar.DisplayDateEnd"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.CalendarDatePicker.SelectedDate"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.CalendarDatePicker.DisplayDate"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDateValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.CalendarDatePicker.DisplayDateStart"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.CalendarDatePicker.DisplayDateEnd"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnCalendarDate",
                IsNullable = true,
            },
            ["Avalonia.Controls.Shapes.Path.Data"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnGeometry",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.Popup.PlacementTarget"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.ContextMenu.PlacementTarget"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.Primitives.PopupFlyoutBase.Popup"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnPopup",
                IsNullable = false,
            },
            ["Avalonia.Controls.Primitives.Popup.Child"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.CommandBar.Content"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.PathIcon.Data"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnGeometry",
                IsNullable = true,
            },
            ["Avalonia.Controls.TableViewColumn.Header"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.TableViewColumn.Width"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = false,
            },
            // The projection has no date/time ABI shape, so a date crosses as an ISO-8601
            // string through a host-side converter, the same mechanism Image.Source uses. The
            // wire form is the invariant round-trip "o" format; reading always produces it and
            // writing accepts any spelling the invariant parser takes, including a bare
            // yyyy-MM-dd. A null or empty string clears SelectedDate.
            ["Avalonia.Controls.DatePicker.SelectedDate"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDateTimeOffset",
                IsNullable = true,
            },
            // MinYear/MaxYear have no absent state, so they take the non-nullable converter:
            // clearing one fails the call rather than silently meaning "today".
            ["Avalonia.Controls.DatePicker.MinYear"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDateTimeOffsetValue",
                IsNullable = false,
            },
            ["Avalonia.Controls.DatePicker.MaxYear"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDateTimeOffsetValue",
                IsNullable = false,
            },
            // A TimePicker selection is a time of day, not a duration, so it crosses as
            // ISO-8601 HH:mm:ss rather than as an ISO-8601 PnDTnHnMnS duration.
            ["Avalonia.Controls.TimePicker.SelectedTime"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnTimeSpan",
                IsNullable = true,
            },
            // Image.Source is an IImage, which is a managed interface with no ABI shape. It
            // crosses as the *source string* the host resolves into a bitmap: an absolute or
            // relative file path, or a file://, avares:// or resm: URI. The host remembers which
            // string produced which bitmap, so a read returns the string the ABI set; an image
            // that came from XAML or from managed code reads back as null.
            ["Avalonia.Controls.Image.Source"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnImageSource",
                IsNullable = true,
            },
            ["Avalonia.Controls.ToggleSwitch.OnContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            ["Avalonia.Controls.ToggleSwitch.OffContent"] = new()
            {
                Kind = MarshallingKind.ComInterface,
                InterfaceName = "Avalonia.Host.Com.IAvnControl",
                IsNullable = true,
            },
            // KeyGesture owns its round-trip: Parse(string) and an overridden ToString().
            ["Avalonia.Controls.Button.HotKey"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = true,
            },
            ["Avalonia.Controls.SplitButton.HotKey"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuItem.HotKey"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = true,
            },
            ["Avalonia.Controls.MenuItem.InputGesture"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                IsNullable = true,
            },
            // ThemeVariant has no Parse/ToString round-trip (ToString yields the key, but
            // only the three well-known variants cross), so a host converter maps the
            // names. A null string means the variant is unset / inherits.
            ["Avalonia.Controls.ThemeVariantScope.RequestedThemeVariant"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnThemeVariant",
                IsNullable = true,
            },
            ["Avalonia.StyledElement.ActualThemeVariant"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnThemeVariant",
                IsNullable = true,
            },
            // BoxShadows prints and parses through its own ToString; the host converter
            // owns both halves because the struct pair has no single static Parse.
            ["Avalonia.Controls.Border.BoxShadow"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnBoxShadows",
                IsNullable = false,
            },
            // TabStripPlacement is a nullable Dock; the converter maps the enum names and
            // maps a null string to the unset placement.
            ["Avalonia.Controls.TabItem.TabStripPlacement"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnDock",
                IsNullable = true,
            },
        },
                    ByDesignMembers = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["ItemTemplate"] = "U19 projects the data templates; container themes are styling",
                        ["ItemContainerTheme"] = "container themes are styling owned by the host",
                        ["ItemsPanel"] = "the items panel is layout plumbing owned by the host",
                        ["ItemContainerGenerator"] = "container generation is owned by the host",
                        ["Presenter"] = "template-part presenters are internal layout plumbing",
                        ["ItemsPanelRoot"] = "the realized panel root is internal layout plumbing",
                        ["ItemsView"] = "the item view is internal plumbing",
                        ["HeaderPresenter"] = "template-part presenters are internal layout plumbing",
                        ["Template"] = "the control template (not a data template) is owned by the host",
                        ["TemplateApplied"] = "control-template plumbing is owned by the host",
                        ["ApplyTemplate"] = "control-template plumbing is owned by the host",
                        ["TemplateSettings"] = "template settings are styling owned by the host",
                        ["DefiningGeometry"] = "geometry internals are owned by the host",
                        ["RenderedGeometry"] = "geometry internals are owned by the host",
                        ["Render"] = "custom drawing is not part of the projected model",
                        ["TextLayout"] = "the text layout engine is not part of the projected model",
                        ["Inlines"] = "the inline text object model is not part of the projected model",
                        ["DataTemplates"] = "the default-template list is owned by the host",
                        ["Styles"] = "the style system is not projected; use the imperative surface",
                        ["Resources"] = "the resource dictionary is not part of the projected model",
                        ["Theme"] = "control themes are styling owned by the host",
                        ["StyleKey"] = "styling is owned by the host",
                        ["PageTransition"] = "transitions are animation internals owned by the host",
                        ["LogicalChildren"] = "the logical tree is owned by the host",
                        ["DisplayMemberBinding"] = "bindings are not projected; use the imperative surface",
                        ["ValueMemberBinding"] = "bindings are not projected; use the imperative surface",
                        ["SelectedValueBinding"] = "bindings are not projected; use the imperative surface",
                        ["FlyoutPresenterTheme"] = "styling is owned by the host",
                        ["FlyoutPresenterClasses"] = "styling is owned by the host",
                        ["IsInsidePopup"] = "visual-tree queries are owned by the host",
                        ["GetRealizedContainers"] = "realized-container enumeration is owned by the host",
                        ["GetRealizedTreeContainers"] = "realized-container enumeration is owned by the host",
                        ["PlatformImpl"] = "the platform implementation is not ABI surface",
                        ["OwnedWindows"] = "window enumeration is owned by the host",
                        ["DependencyResolver"] = "the resolver is not ABI surface",
                        ["ShowDialog"] = "the modal completion needs the dialog-owner lifetime the projected Show overloads do not carry",
                        ["Show"] = "the suppressed override of the base Open slot",
                        ["Close"] = "the suppressed override of the base Close slot",
                        ["Spun"] = "obsolete alias of Spinned",
                        ["OpenedPopups"] = "popup enumeration is owned by the host",
                        ["MaskProvider"] = "mask internals are owned by the host",
                        ["Culture"] = "globalization is owned by the host",
                        ["NumberFormat"] = "globalization is owned by the host",
                        ["ParsingNumberStyle"] = "globalization is owned by the host",
                        ["TextConverter"] = "the converter is owned by the host",
                        ["WindowDecorationsTheme"] = "theming is owned by the host",
                        ["SystemDecorations"] = "decorations are owned by the platform chrome",
                        ["Watermark"] = "obsolete alias of PlaceholderText",
                        ["WatermarkForeground"] = "obsolete alias of PlaceholderForeground",
                        ["UseFloatingWatermark"] = "obsolete alias of UseFloatingPlaceholder",
                        ["TextDecorations"] = "the text decoration model is not part of the projected surface",
                        ["PreparingContainer"] = "container lifecycle is owned by the host",
                        ["ContainerPrepared"] = "container lifecycle is owned by the host",
                        ["ContainerIndexChanged"] = "container lifecycle is owned by the host",
                        ["ContainerClearing"] = "container lifecycle is owned by the host",
                        ["ResourcesChanged"] = "the resource dictionary is not part of the projected model",
                        ["KnobTransitions"] = "animation internals owned by the host",
                        ["Open"] = "the suppressed override of the base Open slot",
                        ["Hide"] = "the sealed override of the projected base Hide slot",
                        ["ContentTransition"] = "transitions are animation internals owned by the host",
                        ["PreviewContent"] = "preview plumbing is owned by the host",
                        ["FocusAdorner"] = "adorner plumbing is owned by the host",
                        ["LayoutTransform"] = "transform internals are owned by the host",
                        ["TransformRoot"] = "transform internals are owned by the host",
                        ["IsItemsHost"] = "layout plumbing owned by the host",
                        ["Visualizer"] = "the visualizer is template plumbing owned by the host",
                        ["PreviousButtonTheme"] = "styling is owned by the host",
                        ["NextButtonTheme"] = "styling is owned by the host",
                        ["HorizontalContentAlignment"] = "the new-hidden member duplicates the projected base",
                        ["VerticalContentAlignment"] = "the new-hidden member duplicates the projected base",
                        ["PointerEnteredItem"] = "internal hover plumbing is owned by the host",
                        ["PointerExitedItem"] = "internal hover plumbing is owned by the host",
                        ["CoerceValue"] = "AvaloniaProperty plumbing is not projected",
                        ["Selection"] = "the selection model is owned by the host",
                        ["Scroll"] = "the scrolling contract is owned by the host",
                        ["SelectionChanged"] = "the added/removed collection payload has no event shape yet",
                        ["Column"] = "the owning column is owned by the host",
                        ["HeaderTheme"] = "styling is owned by the host",
                        ["CellTheme"] = "styling is owned by the host",
                        ["CellTemplate"] = "per-column templates are styling owned by the host",
                        ["Binding"] = "bindings are not projected; use the imperative surface",
                        ["TableView"] = "the owning table is owned by the host",
                        ["HorizontalSnapPointsChanged"] = "snap points are layout plumbing owned by the host",
                        ["VerticalSnapPointsChanged"] = "snap points are layout plumbing owned by the host",
                        ["GetIrregularSnapPoints"] = "snap points are layout plumbing owned by the host",
                        ["GetRegularSnapPoints"] = "snap points are layout plumbing owned by the host",
                        ["ApplyStyling"] = "styling is owned by the host",
                        ["TryGetResource"] = "the resource dictionary is not part of the projected model",
                        ["OnContentPresenter"] = "template-part presenters are internal layout plumbing",
                        ["OffContentPresenter"] = "template-part presenters are internal layout plumbing",
                        ["Menu"] = "the imperative NativeMenu rides the view-model pipeline",
                        ["NativeMenuExporter"] = "internal exporter plumbing",
                        ["Dispose"] = "lifetime is owned by the host",
                        ["BeginMoveDrag"] = "drag needs the pointer-event payload shape",
                        ["BeginResizeDrag"] = "drag needs the pointer-event payload shape",
                        ["SelectedItems"] = "the new-hidden member duplicates the projected base",
                    },
                    EventOverrides = new Dictionary<string, EventProjection>(StringComparer.Ordinal)
        {
            ["Avalonia.Controls.Button.Click"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Primitives.ToggleButton.IsCheckedChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TextBox.TextChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TextBox.CopyingToClipboard"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TextBox.CuttingToClipboard"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TextBox.PastingFromClipboard"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.SelectableTextBlock.CopyingToClipboard"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.ScrollViewer.ScrollChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.ComboBox.DropDownOpened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.ComboBox.DropDownClosed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.PipsPager.SelectedIndexChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.AutoCompleteBox.DropDownOpened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.AutoCompleteBox.DropDownClosed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave R: the old/new decimal pair crosses as invariant UTF-16 strings through
            // the same AvnDecimal converter the Value property already uses.
            ["Avalonia.Controls.NumericUpDown.ValueChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new()
                    {
                        Name = "OldValue",
                        Override = new()
                        {
                            Kind = MarshallingKind.StringUtf16,
                            StringConverterTypeName = "Avalonia.Host.Com.AvnDecimal",
                        },
                    },
                    new()
                    {
                        Name = "NewValue",
                        Override = new()
                        {
                            Kind = MarshallingKind.StringUtf16,
                            StringConverterTypeName = "Avalonia.Host.Com.AvnDecimal",
                        },
                    },
                ],
            },
            ["Avalonia.Controls.CalendarDatePicker.CalendarOpened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.CalendarDatePicker.CalendarClosed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave P: RangeBaseValueChangedEventArgs carries the old/new double pair.
            ["Avalonia.Controls.Primitives.RangeBase.ValueChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "OldValue" }, new() { Name = "NewValue" }],
            },
            ["Avalonia.Controls.Expander.Expanded"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Expander.Collapsed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.CommandBar.Opening"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.CommandBar.Opened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.CommandBar.Closing"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.CommandBar.Closed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave P: SelectionChangedEventArgs carries the added/removed item lists,
            // which cross as Variant slots on the host-implemented args interface.
            ["Avalonia.Controls.Primitives.SelectingItemsControl.SelectionChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters =
                [
                    new() { Name = "AddedItems" },
                    new() { Name = "RemovedItems" },
                ],
            },
            // TreeView derives from ItemsControl, so its SelectionChanged is its own event
            // rather than the SelectingItemsControl one. Same SelectionChangedEventArgs shape.
            ["Avalonia.Controls.TreeView.SelectionChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters =
                [
                    new() { Name = "AddedItems" },
                    new() { Name = "RemovedItems" },
                ],
            },
            ["Avalonia.Controls.TreeViewItem.Expanded"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TreeViewItem.Collapsed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Primitives.FlyoutBase.Opened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Primitives.FlyoutBase.Closed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Primitives.PopupFlyoutBase.Opening"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Closing is the one wave B event with a payload: the handler may veto the close
            // by writing back Cancel, exactly as Control.KeyDown writes back Handled.
            ["Avalonia.Controls.Primitives.PopupFlyoutBase.Closing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.ContextMenu.Opening"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.ContextMenu.Closing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.Expander.Expanding"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.Expander.Collapsing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.RefreshContainer.RefreshRequested"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.TrayIcon.Clicked"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Spinner.Spin"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Direction" }, new() { Name = "UsingMouseWheel" }],
            },
            ["Avalonia.Controls.AutoCompleteBox.TextChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.AutoCompleteBox.Populating"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Cancel", Direction = ParameterDirection.InOut },
                    new() { Name = "Parameter" },
                ],
            },
            ["Avalonia.Controls.AutoCompleteBox.DropDownOpening"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.AutoCompleteBox.DropDownClosing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.Control.SizeChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "NewSize" }, new() { Name = "PreviousSize" }],
            },
            ["Avalonia.StyledElement.DataContextChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.StyledElement.Initialized"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.StyledElement.ActualThemeVariantChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.StyledElement.ResourcesChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.StyledElement.AttachedToLogicalTree"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.StyledElement.DetachedFromLogicalTree"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.NumericUpDown.Spinned"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Direction" }, new() { Name = "UsingMouseWheel" }],
            },
            // U28: the picker value-change events carry the old/new pair; the nullable
            // TimeSpan and DateTimeOffset cross through the optional ABI wrappers.
            ["Avalonia.Controls.TimePicker.SelectedTimeChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "OldTime" }, new() { Name = "NewTime" }],
            },
            ["Avalonia.Controls.DatePicker.SelectedDateChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "OldDate" }, new() { Name = "NewDate" }],
            },
            // TextChangingEventArgs has no payload members, so the event crosses as a
            // plain notification.
            ["Avalonia.Controls.TextBox.TextChanging"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // U31: the transition-completed payload is two variants and a completion
            // flag, which the Fields machinery already carries.
            ["Avalonia.Controls.TransitioningContentControl.TransitionCompleted"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "From" },
                    new() { Name = "To" },
                    new() { Name = "HasRunToCompletion" },
                ],
            },
            // U32: the collection payloads cross as a host-implemented args interface
            // whose count/getter slots expose the items as Variants.
            ["Avalonia.Controls.AutoCompleteBox.SelectionChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters =
                [
                    new() { Name = "AddedItems" },
                    new() { Name = "RemovedItems" },
                ],
            },
            ["Avalonia.Controls.AutoCompleteBox.Populated"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters = [new() { Name = "Data" }],
            },
            ["Avalonia.Controls.Calendar.SelectedDatesChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters =
                [
                    new() { Name = "AddedItems" },
                    new() { Name = "RemovedItems" },
                ],
            },
            ["Avalonia.Controls.CalendarDatePicker.SelectedDateChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Args,
                Parameters =
                [
                    new() { Name = "AddedItems" },
                    new() { Name = "RemovedItems" },
                ],
            },
            ["Avalonia.Controls.CalendarDatePicker.DateValidationError"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Text" },
                    new() { Name = "ThrowException", Direction = ParameterDirection.InOut },
                ],
            },
            ["Avalonia.Controls.SplitView.PaneOpening"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.SplitView.PaneClosing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Cancel", Direction = ParameterDirection.InOut }],
            },
            ["Avalonia.Controls.Calendar.DisplayModeChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "OldMode" }, new() { Name = "NewMode" }],
            },
            ["Avalonia.Controls.Calendar.DisplayDateChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "RemovedDate" }, new() { Name = "AddedDate" }],
            },
            ["Avalonia.Controls.Notifications.NotificationCard.NotificationClosed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Window.Closing"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Cancel", Direction = ParameterDirection.InOut },
                    new() { Name = "CloseReason" },
                    new() { Name = "IsProgrammatic" },
                ],
            },
            ["Avalonia.Controls.Primitives.Thumb.DragStarted"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Vector" }],
            },
            ["Avalonia.Controls.Primitives.Thumb.DragDelta"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Vector" }],
            },
            ["Avalonia.Controls.Primitives.Thumb.DragCompleted"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "Vector" }],
            },
            ["Avalonia.Controls.Primitives.Popup.Opened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Primitives.Popup.Closed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.MenuBase.Opened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.MenuBase.Closed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.MenuItem.Click"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.SplitButton.Click"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.ToggleSplitButton.IsCheckedChanged"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.MenuItem.SubmenuOpened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.SplitView.PaneOpened"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.SplitView.PaneClosed"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Control.KeyDown"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Key" },
                    new() { Name = "PhysicalKey" },
                    new() { Name = "KeyModifiers" },
                    new() { Name = "KeySymbol" },
                    new() { Name = "Handled", Direction = ParameterDirection.InOut },
                ],
            },
            // Wave P: KeyUp mirrors KeyDown; GotFocus carries the navigation context;
            // LostFocusEventArgs has no payload members, so it crosses as a notification.
            ["Avalonia.Controls.Control.KeyUp"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Key" },
                    new() { Name = "PhysicalKey" },
                    new() { Name = "KeyModifiers" },
                    new() { Name = "KeySymbol" },
                    new() { Name = "Handled", Direction = ParameterDirection.InOut },
                ],
            },
            ["Avalonia.Controls.Control.GotFocus"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "NavigationMethod" },
                    new() { Name = "KeyModifiers" },
                ],
            },
            ["Avalonia.Controls.Control.LostFocus"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave Q: PointerEventArgs is a routed-event args type whose host-typed members
            // (IPointer, the relative position) stay off the wire; the device-independent
            // scalars cross as fields.
            ["Avalonia.Controls.Control.PointerEntered"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "KeyModifiers" }],
            },
            ["Avalonia.Controls.Control.PointerExited"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "KeyModifiers" }],
            },
            ["Avalonia.Controls.Control.Loaded"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            ["Avalonia.Controls.Control.Unloaded"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave Q: TappedEventArgs carries the tap as a KeyModifiers field. DoubleTapped
            // has no payload members of its own, so it crosses as a notification.
            ["Avalonia.Controls.Control.Tapped"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters = [new() { Name = "KeyModifiers" }],
            },
            ["Avalonia.Controls.Control.DoubleTapped"] = new()
            {
                PayloadKind = EventPayloadKind.None,
            },
            // Wave Q: PointerWheelEventArgs carries the scroll delta as an AvnVector and
            // the modifiers at the time of the wheel tick.
            ["Avalonia.Controls.Control.PointerWheelChanged"] = new()
            {
                PayloadKind = EventPayloadKind.Fields,
                Parameters =
                [
                    new() { Name = "Delta" },
                    new() { Name = "KeyModifiers" },
                ],
            },
        },
        AttachedProperties = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
        {
            ["Avalonia.Controls.Grid"] =
                ["Column", "Row", "ColumnSpan", "RowSpan", "IsSharedSizeScope"],
            ["Avalonia.Controls.Canvas"] = ["Left", "Top", "Right", "Bottom"],
            ["Avalonia.Controls.DockPanel"] = ["Dock"],
            ["Avalonia.Controls.ToolTip"] =
            [
                "Tip", "IsOpen", "Placement", "HorizontalOffset", "VerticalOffset",
                "ShowDelay", "BetweenShowDelay", "ShowOnDisabled", "ServiceEnabled",
            ],
            ["Avalonia.Controls.RelativePanel"] =
            [
                "AlignLeftWithPanel", "AlignRightWithPanel", "AlignTopWithPanel",
                "AlignBottomWithPanel", "AlignHorizontalCenterWithPanel",
                "AlignVerticalCenterWithPanel",
            ],
        },
        AttachedPropertyOverrides = new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal)
        {
            // ToolTip.Tip is an object so that XAML can hang an arbitrary control off it. Over
            // the ABI it is a string and nothing else: setting one stores the string, and
            // reading one returns null when the tip is a control rather than text. Projecting a
            // control as a tip is a later wave.
            ["Avalonia.Controls.ToolTip.Tip"] = new()
            {
                Kind = MarshallingKind.StringUtf16,
                StringConverterTypeName = "Avalonia.Host.Com.AvnToolTipTip",
                IsNullable = true,
            },
        },
    };
}









