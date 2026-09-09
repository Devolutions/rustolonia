using System;
using System.ComponentModel;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform;
using Avalonia.UnitTests;
using Moq;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the wave B controls: the flyout trio, the imperative menu pair, <see cref="SplitView"/>
/// and the two pickers. Every assertion goes through a real CCW/RCW round trip and then reads the
/// Avalonia object, so a member that only updated wrapper state fails.
/// </summary>
public unsafe class WaveBControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Flyout_content_placement_and_show_mode_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateFlyout(out var projected));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedContent));
        Assert.NotNull(projected);
        Assert.NotNull(projectedContent);

        Through<IAvnFlyout>(projected, flyout =>
        {
            // A flyout is an AvaloniaObject rather than a Control, so it has no layout members;
            // what it does have is content, placement and an open state.
            Assert.Equal(0, flyout.GetContent(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, projectedContent.SetText("Pick one"));
            Assert.Equal(0, flyout.SetContent((IAvnControl)projectedContent));
            Assert.Equal(0, flyout.GetContent(out var content));
            Assert.NotNull(content);

            Assert.Equal(0, flyout.SetPlacement((int)PlacementMode.RightEdgeAlignedTop));
            Assert.Equal(0, flyout.GetPlacement(out var placement));
            Assert.Equal((int)PlacementMode.RightEdgeAlignedTop, placement);

            Assert.Equal(0, flyout.SetShowMode((int)FlyoutShowMode.Transient));
            Assert.Equal(0, flyout.GetShowMode(out var showMode));
            Assert.Equal((int)FlyoutShowMode.Transient, showMode);

            Assert.Equal(0, flyout.SetHorizontalOffset(12.5));
            Assert.Equal(0, flyout.SetVerticalOffset(-3));
            Assert.Equal(0, flyout.SetOverlayDismissEventPassThrough(1));

            // Nothing has been shown yet, so it is closed and has no target.
            Assert.Equal(0, flyout.GetIsOpen(out var isOpen));
            Assert.Equal(0, isOpen);
            Assert.Equal(0, flyout.GetTarget(out var target));
            Assert.Null(target);
        });

        var value = Target<Flyout>(projected);
        Assert.Equal("Pick one", Assert.IsType<TextBlock>(value.Content).Text);
        Assert.Equal(PlacementMode.RightEdgeAlignedTop, value.Placement);
        Assert.Equal(FlyoutShowMode.Transient, value.ShowMode);
        Assert.Equal(12.5, value.HorizontalOffset);
        Assert.Equal(-3, value.VerticalOffset);
        Assert.True(value.OverlayDismissEventPassThrough);
    }

    [Fact]
    public void Showing_a_flyout_at_a_control_opens_it_and_raises_opened_then_closed()
    {
        using var app = CreateWindowedServices();
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateFlyout(out var projected));
        Assert.Equal(0, factory.CreateButton(out var projectedButton));
        Assert.NotNull(projected);
        Assert.NotNull(projectedButton);

        var flyout = Target<Flyout>(projected);
        var button = Target<Button>(projectedButton);
        var window = PreparedWindow(button);
        window.Show();

        var wrapper = Assert.IsType<AvnFlyout>(projected);
        var opened = new CountingHandler();
        var closed = new CountingHandler();
        Assert.Equal(0, wrapper.AdviseOpened(opened, out var openedId));
        Assert.Equal(0, wrapper.AdviseClosed(closed, out var closedId));

        // ShowAt takes any projected control, so the flyout reaches the button without an
        // attached property: a COM-valued attached property has no ABI shape in this wave.
        Assert.Equal(0, wrapper.ShowAtWithControl((IAvnControl)projectedButton));
        Assert.True(flyout.IsOpen);
        Assert.Same(button, flyout.Target);
        Assert.Equal(1, opened.CallCount);

        Assert.Equal(0, wrapper.GetIsOpen(out var isOpen));
        Assert.Equal(1, isOpen);
        Assert.Equal(0, wrapper.GetTarget(out var target));
        Assert.NotNull(target);

        Assert.Equal(0, wrapper.Hide());
        Assert.False(flyout.IsOpen);
        Assert.Equal(1, closed.CallCount);

        Assert.Equal(0, wrapper.UnadviseOpened(openedId));
        Assert.Equal(0, wrapper.UnadviseClosed(closedId));
        Assert.True(wrapper.UnadviseClosed(closedId) < 0);
    }

    [Fact]
    public void A_closing_handler_can_veto_the_close_by_writing_cancel_back()
    {
        using var app = CreateWindowedServices();
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateFlyout(out var projected));
        Assert.Equal(0, factory.CreateButton(out var projectedButton));
        Assert.NotNull(projected);
        Assert.NotNull(projectedButton);

        var flyout = Target<Flyout>(projected);
        var window = PreparedWindow(Target<Button>(projectedButton));
        window.Show();

        var wrapper = Assert.IsType<AvnFlyout>(projected);
        var closing = new VetoingClosingHandler { Cancel = true };
        Assert.Equal(0, wrapper.AdviseClosing(closing, out var closingId));
        Assert.Equal(0, wrapper.ShowAtWithControl((IAvnControl)projectedButton));

        Assert.Equal(0, wrapper.Hide());
        Assert.Equal(1, closing.CallCount);
        // Cancel is an in/out field, exactly like KeyDown's Handled, so writing it back keeps
        // the flyout open.
        Assert.True(flyout.IsOpen);

        closing.Cancel = false;
        Assert.Equal(0, wrapper.Hide());
        Assert.Equal(2, closing.CallCount);
        Assert.False(flyout.IsOpen);
        Assert.Equal(0, wrapper.UnadviseClosing(closingId));
    }

    [Fact]
    public void Showing_a_flyout_at_no_control_fails_the_call()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateFlyout(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnFlyout>(projected);
        Assert.True(wrapper.ShowAtWithControl(null!) < 0);
        Assert.False(Target<Flyout>(projected).IsOpen);

        // Hiding a flyout that was never shown is a no-op rather than an error.
        Assert.Equal(0, wrapper.Hide());
    }

    [Fact]
    public void Menu_opens_and_closes_imperatively_and_raises_its_events()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMenu(out var projected));
        Assert.Equal(0, factory.CreateMenuItem(out var projectedItem));
        Assert.NotNull(projected);
        Assert.NotNull(projectedItem);

        var menu = Target<Menu>(projected);
        var wrapper = Assert.IsType<AvnMenu>(projected);
        var opened = new CountingHandler();
        var closed = new CountingHandler();
        Assert.Equal(0, wrapper.AdviseOpened(opened, out _));
        Assert.Equal(0, wrapper.AdviseClosed(closed, out _));

        Through<IAvnMenu>(projected, projectedMenu =>
        {
            // Items comes from IAvnItemsControl: a Menu is an imperative ItemsControl, not the
            // view-model NativeMenu.
            Assert.Equal(0, projectedMenu.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedItem));

            Assert.Equal(0, projectedMenu.GetIsOpen(out var initial));
            Assert.Equal(0, initial);

            Assert.Equal(0, projectedMenu.Open());
            Assert.Equal(0, projectedMenu.GetIsOpen(out var isOpen));
            Assert.Equal(1, isOpen);

            Assert.Equal(0, projectedMenu.Close());
            Assert.Equal(0, projectedMenu.GetIsOpen(out var reclosed));
            Assert.Equal(0, reclosed);
        });

        Assert.False(menu.IsOpen);
        Assert.Single(menu.Items);
        Assert.Equal(1, opened.CallCount);
        Assert.Equal(1, closed.CallCount);
    }

    [Fact]
    public void Menu_item_header_icon_and_toggle_state_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMenuItem(out var projected));
        Assert.Equal(0, factory.CreateMenuItem(out var projectedChild));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedHeader));
        Assert.Equal(0, factory.CreateImage(out var projectedIcon));
        Assert.NotNull(projected);
        Assert.NotNull(projectedChild);
        Assert.NotNull(projectedHeader);
        Assert.NotNull(projectedIcon);

        Through<IAvnMenuItem>(projected, item =>
        {
            Assert.Equal(0, projectedHeader.SetText("Save"));
            // Header comes from the newly projected HeaderedSelectingItemsControl.
            Assert.Equal(0, item.SetHeader((IAvnControl)projectedHeader));
            Assert.Equal(0, item.GetHeader(out var header));
            Assert.NotNull(header);

            Assert.Equal(0, item.SetIcon((IAvnControl)projectedIcon));
            Assert.Equal(0, item.GetIcon(out var icon));
            Assert.NotNull(icon);

            Assert.Equal(0, item.SetToggleType((int)MenuItemToggleType.CheckBox));
            Assert.Equal(0, item.SetIsChecked(1));
            Assert.Equal(0, item.GetIsChecked(out var isChecked));
            Assert.Equal(1, isChecked);

            Assert.Equal(0, item.SetStaysOpenOnClick(1));
            Assert.Equal(0, item.SetGroupName("edits"));
            Assert.Equal(0, item.GetGroupName(out var groupName));
            Assert.Equal("edits", groupName);

            // IsEnabled is inherited from IAvnControl rather than redeclared.
            Assert.Equal(0, item.SetIsEnabled(0));

            Assert.Equal(0, item.GetItems(out var items));
            Assert.NotNull(items);
            Assert.Equal(0, items.Add((IAvnControl)projectedChild));
        });

        var value = Target<MenuItem>(projected);
        Assert.Equal("Save", Assert.IsType<TextBlock>(value.Header).Text);
        Assert.IsType<Image>(value.Icon);
        Assert.Equal(MenuItemToggleType.CheckBox, value.ToggleType);
        Assert.True(value.IsChecked);
        Assert.True(value.StaysOpenOnClick);
        Assert.Equal("edits", value.GroupName);
        Assert.False(value.IsEnabled);
        Assert.Single(value.Items);
    }

    [Fact]
    public void Menu_item_click_bridges_to_the_abi_without_an_icommand()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMenuItem(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnMenuItem>(projected);
        var value = Target<MenuItem>(projected);
        var click = new CountingHandler();
        Assert.Equal(0, wrapper.AdviseClick(click, out var clickId));

        value.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent, value));
        Assert.Equal(1, click.CallCount);

        Assert.Equal(0, wrapper.UnadviseClick(clickId));
        value.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent, value));
        Assert.Equal(1, click.CallCount);
    }

    [Fact]
    public void Split_view_pane_and_content_cross_as_controls_and_the_pane_events_fire()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateSplitView(out var projected));
        Assert.Equal(0, factory.CreateStackPanel(out var projectedPane));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedContent));
        Assert.Equal(0, factory.CreateSolidColorBrush(
            AvnColor.FromAvalonia(Media.Color.FromArgb(0xFF, 0x10, 0x20, 0x30)),
            1,
            out var brush));
        Assert.NotNull(projected);
        Assert.NotNull(projectedPane);
        Assert.NotNull(projectedContent);
        Assert.NotNull(brush);

        var wrapper = Assert.IsType<AvnSplitView>(projected);
        var opened = new CountingHandler();
        var closed = new CountingHandler();
        Assert.Equal(0, wrapper.AdvisePaneOpened(opened, out _));
        Assert.Equal(0, wrapper.AdvisePaneClosed(closed, out _));

        Through<IAvnSplitView>(projected, splitView =>
        {
            Assert.Equal(0, splitView.GetPane(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, splitView.SetPane((IAvnControl)projectedPane));
            Assert.Equal(0, splitView.GetPane(out var pane));
            Assert.NotNull(pane);

            // Content is inherited from IAvnContentControl.
            Assert.Equal(0, splitView.SetContent((IAvnControl)projectedContent));

            Assert.Equal(0, splitView.SetDisplayMode((int)SplitViewDisplayMode.CompactOverlay));
            Assert.Equal(0, splitView.GetDisplayMode(out var displayMode));
            Assert.Equal((int)SplitViewDisplayMode.CompactOverlay, displayMode);

            Assert.Equal(0, splitView.SetPanePlacement((int)SplitViewPanePlacement.Right));
            Assert.Equal(0, splitView.SetOpenPaneLength(280));
            Assert.Equal(0, splitView.SetCompactPaneLength(44));
            Assert.Equal(0, splitView.SetUseLightDismissOverlayMode(1));
            Assert.Equal(0, splitView.SetPaneBackground(brush));

            Assert.Equal(0, splitView.SetIsPaneOpen(1));
            Assert.Equal(0, splitView.GetIsPaneOpen(out var isPaneOpen));
            Assert.Equal(1, isPaneOpen);
            Assert.Equal(0, splitView.SetIsPaneOpen(0));
        });

        var value = Target<SplitView>(projected);
        Assert.IsType<StackPanel>(value.Pane);
        Assert.IsType<TextBlock>(value.Content);
        Assert.Equal(SplitViewDisplayMode.CompactOverlay, value.DisplayMode);
        Assert.Equal(SplitViewPanePlacement.Right, value.PanePlacement);
        Assert.Equal(280, value.OpenPaneLength);
        Assert.Equal(44, value.CompactPaneLength);
        Assert.True(value.UseLightDismissOverlayMode);
        Assert.False(value.IsPaneOpen);
        Assert.Equal(1, opened.CallCount);
        Assert.Equal(1, closed.CallCount);
    }

    [Fact]
    public void Date_picker_dates_cross_as_invariant_iso_8601_strings()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateDatePicker(out var projected));
        Assert.NotNull(projected);

        Through<IAvnDatePicker>(projected, picker =>
        {
            // No selection is a null, not an empty string and not a default date.
            Assert.Equal(0, picker.GetSelectedDate(out var initial));
            Assert.Null(initial);

            // Writing accepts any spelling the invariant parser takes, including a bare date.
            Assert.Equal(0, picker.SetSelectedDate("2026-09-03"));
            Assert.Equal(0, picker.GetSelectedDate(out var selected));
            // Reading always produces the round-trip "o" form, so it is not what was written.
            Assert.NotNull(selected);
            Assert.StartsWith("2026-09-03T00:00:00.0000000", selected);

            Assert.Equal(0, picker.SetSelectedDate("2027-01-15T08:30:00.0000000+02:00"));
            Assert.Equal(0, picker.GetSelectedDate(out var offsetForm));
            Assert.Equal("2027-01-15T08:30:00.0000000+02:00", offsetForm);

            Assert.Equal(0, picker.SetMinYear("2000-01-01T00:00:00.0000000+00:00"));
            Assert.Equal(0, picker.GetMinYear(out var minYear));
            Assert.Equal("2000-01-01T00:00:00.0000000+00:00", minYear);
            Assert.Equal(0, picker.SetMaxYear("2100-12-31T00:00:00.0000000+00:00"));

            Assert.Equal(0, picker.SetDayVisible(0));
            Assert.Equal(0, picker.SetMonthFormat("MMMM"));
            Assert.Equal(0, picker.GetMonthFormat(out var monthFormat));
            Assert.Equal("MMMM", monthFormat);

            // An empty string clears the selection; there is no "empty date" state.
            Assert.Equal(0, picker.SetSelectedDate(null));
            Assert.Equal(0, picker.GetSelectedDate(out var cleared));
            Assert.Null(cleared);
            Assert.Equal(0, picker.Clear());
        });

        var value = Target<DatePicker>(projected);
        Assert.Null(value.SelectedDate);
        Assert.Equal(new DateTimeOffset(2000, 1, 1, 0, 0, 0, TimeSpan.Zero), value.MinYear);
        Assert.Equal(new DateTimeOffset(2100, 12, 31, 0, 0, 0, TimeSpan.Zero), value.MaxYear);
        Assert.False(value.DayVisible);
        Assert.Equal("MMMM", value.MonthFormat);
    }

    [Fact]
    public void A_date_that_is_not_iso_8601_fails_the_call_and_min_year_cannot_be_cleared()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateDatePicker(out var projected));
        Assert.NotNull(projected);
        var value = Target<DatePicker>(projected);
        var minYear = value.MinYear;

        Through<IAvnDatePicker>(projected, picker =>
        {
            // A locale spelling is rejected rather than guessed at: 03/09/2026 is ambiguous.
            Assert.True(picker.SetSelectedDate("03/09/2026") < 0);
            Assert.True(picker.SetSelectedDate("not a date") < 0);
            // MinYear has no absent state, so clearing it fails instead of meaning "today".
            Assert.True(picker.SetMinYear(null!) < 0);
            Assert.True(picker.SetMinYear(string.Empty) < 0);
        });

        Assert.Null(value.SelectedDate);
        Assert.Equal(minYear, value.MinYear);
    }

    [Fact]
    public void Time_picker_selection_crosses_as_a_24_hour_time_of_day()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTimePicker(out var projected));
        Assert.NotNull(projected);

        Through<IAvnTimePicker>(projected, picker =>
        {
            Assert.Equal(0, picker.GetSelectedTime(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, picker.SetSelectedTime("17:04"));
            Assert.Equal(0, picker.GetSelectedTime(out var selected));
            // Reading always produces HH:mm:ss even when HH:mm was written.
            Assert.Equal("17:04:00", selected);

            Assert.Equal(0, picker.SetSelectedTime("08:15:30"));
            Assert.Equal(0, picker.GetSelectedTime(out var withSeconds));
            Assert.Equal("08:15:30", withSeconds);

            Assert.Equal(0, picker.SetMinuteIncrement(15));
            Assert.Equal(0, picker.SetUseSeconds(1));
            Assert.Equal(0, picker.SetClockIdentifier("24HourClock"));
            Assert.Equal(0, picker.GetClockIdentifier(out var clock));
            Assert.Equal("24HourClock", clock);

            // A duration spelling is not a time of day, and neither is a negative one.
            Assert.True(picker.SetSelectedTime("PT8H15M") < 0);
            Assert.True(picker.SetSelectedTime("-01:00:00") < 0);
            Assert.True(picker.SetSelectedTime("25:00:00") < 0);

            Assert.Equal(0, picker.SetSelectedTime(string.Empty));
            Assert.Equal(0, picker.GetSelectedTime(out var cleared));
            Assert.Null(cleared);
        });

        var value = Target<TimePicker>(projected);
        Assert.Null(value.SelectedTime);
        Assert.Equal(15, value.MinuteIncrement);
        Assert.True(value.UseSeconds);
        Assert.Equal("24HourClock", value.ClockIdentifier);
    }

    [Fact]
    public void A_time_the_managed_side_set_outside_a_day_fails_the_read_rather_than_wrapping()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTimePicker(out var projected));
        Assert.NotNull(projected);

        // Managed code may store any TimeSpan. The ABI carries a time of day, so a span that is
        // not one fails the read instead of being truncated into a plausible-looking time.
        Target<TimePicker>(projected).SelectedTime = TimeSpan.FromHours(26);
        Through<IAvnTimePicker>(projected, picker =>
            Assert.True(picker.GetSelectedTime(out _) < 0));
    }

    private sealed class CountingHandler :
        IAvnFlyoutBaseOpenedHandler,
        IAvnFlyoutBaseClosedHandler,
        IAvnMenuBaseOpenedHandler,
        IAvnMenuBaseClosedHandler,
        IAvnMenuItemClickHandler,
        IAvnSplitViewPaneOpenedHandler,
        IAvnSplitViewPaneClosedHandler
    {
        public int CallCount { get; private set; }

        public int Invoke()
        {
            CallCount++;
            return 0;
        }
    }

    private sealed class VetoingClosingHandler : IAvnPopupFlyoutBaseClosingHandler
    {
        public int CallCount { get; private set; }

        public bool Cancel { get; set; }

        public int Invoke(ref int cancel)
        {
            CallCount++;
            cancel = Cancel ? 1 : 0;
            return 0;
        }
    }

    private static IDisposable CreateWindowedServices() =>
        // FocusableWindow is StyledWindow plus a real keyboard device, and its
        // MockWindowingPlatform already hands out popup mocks, which is what a shown flyout
        // needs.
        UnitTestApplication.Start(TestServices.FocusableWindow);

    private static Window PreparedWindow(object? content = null)
    {
        var platform = AvaloniaLocator.Current.GetRequiredService<IWindowingPlatform>();
        var windowImpl = Mock.Get(platform.CreateWindow());
        windowImpl.Setup(x => x.Compositor).Returns(RendererMocks.CreateDummyCompositor());
        var window = new Window(windowImpl.Object) { Content = content };
        window.ApplyTemplate();
        return window;
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
