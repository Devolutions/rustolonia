using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU15AutoCompleteComTests
{
    [Fact]
    public void Minimum_populate_delay_round_trips_as_ticks()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);
        Assert.Equal(0, wrapper.SetMinimumPopulateDelay(250_0000L));
        Assert.Equal(0, wrapper.GetMinimumPopulateDelay(out var ticks));
        Assert.Equal(250_0000L, ticks);
    }

    [Fact]
    public void Auto_complete_events_advise_and_unadvise()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnAutoCompleteBox>(projected);

        Assert.Equal(0, wrapper.AdvisePopulating(new PopulatingHandler(), out var populating));
        Assert.Equal(0, wrapper.AdviseDropDownOpening(new CancelHandler(), out var opening));
        Assert.Equal(0, wrapper.AdviseDropDownClosing(new CancelHandler(), out var closing));
        Assert.Equal(0, wrapper.AdviseTextChanged(new TextHandler(), out var changed));

        Assert.Equal(0, wrapper.UnadvisePopulating(populating));
        Assert.Equal(0, wrapper.UnadviseDropDownOpening(opening));
        Assert.Equal(0, wrapper.UnadviseDropDownClosing(closing));
        Assert.Equal(0, wrapper.UnadviseTextChanged(changed));
    }

    private sealed class PopulatingHandler : IAvnAutoCompleteBoxPopulatingHandler
    {
        public int Invoke(ref int Cancel, string? Parameter) => 0;
    }

    private sealed class CancelHandler : IAvnAutoCompleteBoxDropDownOpeningHandler, IAvnAutoCompleteBoxDropDownClosingHandler
    {
        public int Invoke(ref int Cancel) => 0;
    }

    private sealed class TextHandler : IAvnAutoCompleteBoxTextChangedHandler
    {
        public int Invoke() => 0;
    }
}
