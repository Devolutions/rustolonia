using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveFControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Calendar_days_cross_as_yyyy_mm_dd()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendar(out var projected));
        Assert.NotNull(projected);

        Through<IAvnCalendar>(projected, calendar =>
        {
            Assert.Equal(0, calendar.SetDisplayDate("2027-01-15"));
            Assert.Equal(0, calendar.GetDisplayDate(out var display));
            Assert.Equal("2027-01-15", display);

            Assert.Equal(0, calendar.SetSelectedDate("2027-01-20"));
            Assert.Equal(0, calendar.GetSelectedDate(out var selected));
            Assert.Equal("2027-01-20", selected);

            Assert.Equal(0, calendar.SetDisplayMode((int)CalendarMode.Year));
            Assert.Equal(0, calendar.SetSelectionMode((int)CalendarSelectionMode.SingleDate));
            Assert.True(calendar.SetSelectedDate("03/09/2026") < 0);
            Assert.Equal(0, calendar.SetSelectedDate(string.Empty));
            Assert.Equal(0, calendar.GetSelectedDate(out var cleared));
            Assert.Null(cleared);
            Assert.True(calendar.SetDisplayDate("") < 0);
        });

        var value = Target<Calendar>(projected);
        Assert.Null(value.SelectedDate);
        Assert.Equal(new DateTime(2027, 1, 15), value.DisplayDate.Date);
        Assert.Equal(CalendarMode.Year, value.DisplayMode);
    }

    [Fact]
    public void Calendar_date_picker_selection_crosses_as_a_calendar_day()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendarDatePicker(out var projected));
        Assert.NotNull(projected);

        Through<IAvnCalendarDatePicker>(projected, picker =>
        {
            Assert.Equal(0, picker.SetSelectedDate("2026-09-03"));
            Assert.Equal(0, picker.GetSelectedDate(out var selected));
            Assert.Equal("2026-09-03", selected);
            Assert.Equal(0, picker.SetPlaceholderText("Pick a day"));
            Assert.Equal(0, picker.SetIsTodayHighlighted(0));
        });

        var value = Target<CalendarDatePicker>(projected);
        Assert.Equal(new DateTime(2026, 9, 3), value.SelectedDate!.Value.Date);
        Assert.Equal("Pick a day", value.PlaceholderText);
        Assert.False(value.IsTodayHighlighted);
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
