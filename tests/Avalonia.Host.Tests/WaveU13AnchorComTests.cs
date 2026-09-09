using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU13AnchorComTests
{
    [Fact]
    public void Command_bar_events_advise_and_unadvise()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCommandBar(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnCommandBar>(projected);

        Assert.Equal(0, wrapper.AdviseOpened(new CountHandler(), out var openedSubscription));
        Assert.Equal(0, wrapper.AdviseClosing(new CountHandler(), out var closingSubscription));
        Assert.NotEqual(0, openedSubscription);
        Assert.NotEqual(0, closingSubscription);

        Assert.Equal(0, wrapper.UnadviseOpened(openedSubscription));
        Assert.True(wrapper.UnadviseOpened(openedSubscription) < 0);
        Assert.Equal(0, wrapper.UnadviseClosing(closingSubscription));
        Assert.True(wrapper.UnadviseClosing(closingSubscription) < 0);
    }

    [Fact]
    public void Scroll_viewer_registers_anchor_candidates_and_reports_vector_maximum()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateScrollViewer(out var viewer));
        Assert.NotNull(viewer);
        Assert.Equal(0, factory.CreateButton(out var button));
        Assert.NotNull(button);

        var scrollWrapper = Assert.IsType<AvnScrollViewer>(viewer);
        var buttonWrapper = Assert.IsType<AvnButton>(button);

        Assert.Equal(0, scrollWrapper.RegisterAnchorCandidateWithControl(buttonWrapper));
        Assert.Equal(0, scrollWrapper.UnregisterAnchorCandidateWithControl(buttonWrapper));
        Assert.Equal(0, scrollWrapper.UnregisterAnchorCandidateWithControl(buttonWrapper));

        Assert.Equal(0, scrollWrapper.GetScrollBarMaximum(out var maximum));
        Assert.Equal(0.0, maximum.X);
        Assert.Equal(0.0, maximum.Y);
    }

    private sealed class CountHandler : IAvnCommandBarOpenedHandler, IAvnCommandBarClosingHandler
    {
        public int Invoke() => 0;
    }
}
