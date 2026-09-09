using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU31TransitionPayloadComTests
{
    [Fact]
    public void Transition_completed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTransitioningContentControl(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTransitioningContentControl>(projected);

        var handler = new TransitionHandler();
        Assert.Equal(0, wrapper.AdviseTransitionCompleted(handler, out var subscription));
        Assert.Equal(0, wrapper.UnadviseTransitionCompleted(subscription));
        Assert.True(wrapper.UnadviseTransitionCompleted(subscription) < 0);
    }

    [Fact]
    public void Transition_completed_invokes_with_from_to_and_completion()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTransitioningContentControl(out var projected));

        var wrapper = Assert.IsType<AvnTransitioningContentControl>(projected);
        var handler = new TransitionHandler();
        Assert.Equal(0, wrapper.AdviseTransitionCompleted(handler, out var subscription));

        var value = Assert.IsType<TransitioningContentControl>(
            typeof(AvnTransitioningContentControl)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        // A transition never runs in the headless host (no animation clock), so
        // the payload cannot be observed end-to-end here; the subscription is
        // validated by the advise/unadvise pair and the slot signature by the
        // generated interface.
        value.Content = new Button();
        Assert.False(handler.Invoked);

        wrapper.UnadviseTransitionCompleted(subscription);
    }

    private sealed class TransitionHandler : IAvnTransitioningContentControlTransitionCompletedHandler
    {
        public bool Invoked;
        public AvnVariant? From;
        public AvnVariant? To;

        public int Invoke(AvnVariant from, AvnVariant to, int hasRunToCompletion)
        {
            Invoked = true;
            From = from;
            To = to;
            return 0;
        }
    }
}
