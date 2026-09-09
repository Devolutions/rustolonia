using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU16LifecycleComTests
{
    [Fact]
    public void Size_changed_advises_and_fires_with_both_sizes()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnBorder>(projected);
        var handler = new SizeHandler();

        Assert.Equal(0, wrapper.AdviseSizeChanged(handler, out var subscription));

        var value = Assert.IsType<Border>(
            typeof(AvnBorder)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        value.RaiseEvent(new Avalonia.Controls.SizeChangedEventArgs(Avalonia.Controls.Control.SizeChangedEvent)
        {
            NewSize = new Avalonia.Size(120, 80),
            PreviousSize = new Avalonia.Size(60, 40),
            Source = value,
        });

        Assert.Equal(1, handler.CallCount);
        Assert.Equal(120, handler.NewSize.Width);
        Assert.Equal(80, handler.NewSize.Height);
        Assert.Equal(60, handler.PreviousSize.Width);
        Assert.Equal(40, handler.PreviousSize.Height);

        Assert.Equal(0, wrapper.UnadviseSizeChanged(subscription));
    }

    [Fact]
    public void Data_context_changed_advises_and_unadvises()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnBorder>(projected);

        Assert.Equal(0, wrapper.AdviseDataContextChanged(new ContextHandler(), out var subscription));
        Assert.Equal(0, wrapper.UnadviseDataContextChanged(subscription));
        Assert.True(wrapper.UnadviseDataContextChanged(subscription) < 0);
    }

    private sealed class SizeHandler : IAvnControlSizeChangedHandler
    {
        public int CallCount { get; private set; }
        public Avalonia.Size NewSize { get; private set; }
        public Avalonia.Size PreviousSize { get; private set; }

        public int Invoke(AvnSize newSize, AvnSize previousSize)
        {
            CallCount++;
            NewSize = newSize.ToAvalonia();
            PreviousSize = previousSize.ToAvalonia();
            return 0;
        }
    }

    private sealed class ContextHandler : IAvnStyledElementDataContextChangedHandler
    {
        public int Invoke() => 0;
    }
}
