using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU32CollectionPayloadComTests
{
    [Fact]
    public void Calendar_selected_dates_changed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendar(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCalendar>(projected);

        var handler = new DatesChangedHandler();
        Assert.Equal(0, wrapper.AdviseSelectedDatesChanged(handler, out var subscription));
        Assert.Equal(0, wrapper.UnadviseSelectedDatesChanged(subscription));
        Assert.True(wrapper.UnadviseSelectedDatesChanged(subscription) < 0);
    }

    [Fact]
    public void Calendar_selected_dates_changed_carries_the_collections()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCalendar(out var projected));

        var wrapper = Assert.IsType<AvnCalendar>(projected);
        var handler = new DatesChangedHandler();
        Assert.Equal(0, wrapper.AdviseSelectedDatesChanged(handler, out var subscription));

        var value = Assert.IsType<Calendar>(
            typeof(AvnCalendar)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        value.SelectedDates.Add(new DateTime(2024, 6, 15));
        Assert.True(handler.Invoked);
        Assert.Equal(1, handler.AddedCount);
        Assert.Equal(0, handler.RemovedCount);

        wrapper.UnadviseSelectedDatesChanged(subscription);
    }

    private sealed class DatesChangedHandler : IAvnCalendarSelectedDatesChangedHandler
    {
        public bool Invoked;
        public int AddedCount;
        public int RemovedCount;

        public int Invoke(IAvnCalendarSelectedDatesChangedArgs args)
        {
            Invoked = true;
            if (args.GetAddedItemsCount(out var added) < 0 ||
                args.GetRemovedItemsCount(out var removed) < 0)
                return -2147467259;
            AddedCount = added;
            RemovedCount = removed;
            return 0;
        }
    }
}
