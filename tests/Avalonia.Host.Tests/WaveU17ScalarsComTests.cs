using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU17ScalarsComTests
{
    [Fact]
    public void Window_position_round_trips_as_a_pixel_point()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindow(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnWindow>(projected);

        Assert.Equal(0, wrapper.GetPosition(out var before));
        Assert.Equal(0, wrapper.SetPosition(new AvnPixelPoint { X = 120, Y = 80 }));
        Assert.Equal(0, wrapper.GetPosition(out var after));
        Assert.Equal(120, after.X);
        Assert.Equal(80, after.Y);
    }

    [Fact]
    public void Text_box_caret_blink_interval_round_trips_as_ticks()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTextBox>(projected);

        Assert.Equal(0, wrapper.SetCaretBlinkInterval(500_0000L));
        Assert.Equal(0, wrapper.GetCaretBlinkInterval(out var ticks));
        Assert.Equal(500_0000L, ticks);
    }

    [Fact]
    public void Items_control_maps_containers_by_index()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnListBox>(projected);

        // No items are realized in the unit-test host, so the mapping reports
        // null/-1; the slots themselves must round-trip.
        Assert.Equal(0, wrapper.ContainerFromIndexWithInt32(0, out var container));
        Assert.Null(container);
        Assert.Equal(0, factory.CreateListBoxItem(out var item));
        var itemWrapper = Assert.IsType<AvnListBoxItem>(item);
        Assert.Equal(0, wrapper.IndexFromContainerWithControl(itemWrapper, out var index));
        Assert.Equal(-1, index);
    }

    [Fact]
    public void Calendar_display_date_changed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendar(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCalendar>(projected);

        Assert.Equal(0, wrapper.AdviseDisplayDateChanged(new DateHandler(), out var subscription));
        Assert.Equal(0, wrapper.UnadviseDisplayDateChanged(subscription));
        Assert.True(wrapper.UnadviseDisplayDateChanged(subscription) < 0);
    }

    private sealed class DateHandler : IAvnCalendarDisplayDateChangedHandler
    {
        public int Invoke(AvnOptionalDateTime RemovedDate, AvnOptionalDateTime AddedDate) => 0;
    }
}
