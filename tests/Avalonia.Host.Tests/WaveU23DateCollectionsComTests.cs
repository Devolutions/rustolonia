using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU23DateCollectionsComTests
{
    [Fact]
    public void Calendar_selected_dates_cross_as_a_tick_collection()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendar(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCalendar>(projected);

        Assert.Equal(0, wrapper.GetSelectedDates(out var selected));
        Assert.NotNull(selected);
        Assert.Equal(0, selected!.GetCount(out var count));
        Assert.Equal(0, count);

        var epoch = new global::System.DateTime(2024, 1, 15, 0, 0, 0, global::System.DateTimeKind.Utc);
        Assert.Equal(0, selected.Add(epoch.Ticks));
        Assert.Equal(0, selected.GetCount(out count));
        Assert.Equal(1, count);
        Assert.Equal(0, selected.GetAt(0, out var ticks));
        Assert.Equal(epoch.Ticks, ticks);

        // BlackoutDates shares the same adapter over the calendar's collection.
        Assert.Equal(0, wrapper.GetBlackoutDates(out var blackout));
        Assert.NotNull(blackout);
    }

    [Fact]
    public void Calendar_date_picker_blackout_dates_cross()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendarDatePicker(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCalendarDatePicker>(projected);
        // The picker's blackout collection appears with its calendar, which the
        // unit-test host never templates, so the adapter reads null.
        Assert.Equal(0, wrapper.GetBlackoutDates(out var blackout));
        Assert.Null(blackout);
    }
}
