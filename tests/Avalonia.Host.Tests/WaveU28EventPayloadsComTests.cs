using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU28EventPayloadsComTests
{
    [Fact]
    public void Date_picker_selected_date_changed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateDatePicker(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnDatePicker>(projected);

        var handler = new DateChangedHandler();
        Assert.Equal(0, wrapper.AdviseSelectedDateChanged(handler, out var subscription));
        Assert.Equal(0, wrapper.UnadviseSelectedDateChanged(subscription));
        Assert.True(wrapper.UnadviseSelectedDateChanged(subscription) < 0);
    }

    [Fact]
    public void Date_picker_selected_date_changed_invokes_with_old_and_new_dates()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateDatePicker(out var projected));

        var wrapper = Assert.IsType<AvnDatePicker>(projected);
        var handler = new DateChangedHandler();
        Assert.Equal(0, wrapper.AdviseSelectedDateChanged(handler, out var subscription));

        var value = Assert.IsType<DatePicker>(
            typeof(AvnDatePicker)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var before = value.SelectedDate;
        value.SelectedDate = new DateTimeOffset(2024, 6, 15, 0, 0, 0, TimeSpan.Zero);
        Assert.True(handler.Invoked);
        Assert.Equal(before, handler.OldDate?.ToDateTime());
        Assert.Equal(2024, handler.NewDate?.ToDateTime()?.Year);

        wrapper.UnadviseSelectedDateChanged(subscription);
    }

    [Fact]
    public void Time_picker_selected_time_changed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTimePicker(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTimePicker>(projected);

        var handler = new TimeChangedHandler();
        Assert.Equal(0, wrapper.AdviseSelectedTimeChanged(handler, out var subscription));
        Assert.Equal(0, wrapper.UnadviseSelectedTimeChanged(subscription));
        Assert.True(wrapper.UnadviseSelectedTimeChanged(subscription) < 0);
    }

    [Fact]
    public void Text_box_text_changing_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTextBox>(projected);

        var handler = new TextChangingHandler();
        Assert.Equal(0, wrapper.AdviseTextChanging(handler, out var subscription));
        Assert.Equal(0, wrapper.UnadviseTextChanging(subscription));
        Assert.True(wrapper.UnadviseTextChanging(subscription) < 0);
    }

    private sealed class DateChangedHandler : IAvnDatePickerSelectedDateChangedHandler
    {
        public bool Invoked;
        public AvnOptionalDateTime? OldDate;
        public AvnOptionalDateTime? NewDate;

        public int Invoke(AvnOptionalDateTime oldDate, AvnOptionalDateTime newDate)
        {
            Invoked = true;
            OldDate = oldDate;
            NewDate = newDate;
            return 0;
        }
    }

    private sealed class TimeChangedHandler : IAvnTimePickerSelectedTimeChangedHandler
    {
        public int Invoke(AvnOptionalTimeSpan oldTime, AvnOptionalTimeSpan newTime) => 0;
    }

    private sealed class TextChangingHandler : IAvnTextBoxTextChangingHandler
    {
        public int Invoke() => 0;
    }
}
