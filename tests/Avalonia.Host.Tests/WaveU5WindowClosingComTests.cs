using System;
using System.Reflection;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU5WindowClosingComTests
{
    [Fact]
    public void Closing_handler_can_veto_window_close_by_writing_cancel_back()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindow(out var projected));
        Assert.NotNull(projected);

        var window = Target<Window>(projected);
        var closed = 0;
        window.Closed += (_, _) => closed++;
        var handler = new VetoingWindowClosingHandler { Cancel = true };
        Assert.Equal(0, ((IAvnWindow)projected).AdviseClosing(handler, out var id));

        window.Close();
        Assert.Equal(1, handler.CallCount);
        Assert.Equal(0, closed);

        handler.Cancel = false;
        window.Close();
        Assert.Equal(2, handler.CallCount);
        Assert.Equal(1, closed);
        Assert.Equal(0, ((IAvnWindow)projected).UnadviseClosing(id));
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));

    private sealed class VetoingWindowClosingHandler : IAvnWindowClosingHandler
    {
        public bool Cancel { get; set; }
        public int CallCount { get; private set; }

        public int Invoke(ref int cancel, int closeReason, int isProgrammatic)
        {
            CallCount++;
            Assert.True(closeReason >= 0);
            Assert.True(isProgrammatic is 0 or 1);
            cancel = Cancel ? 1 : 0;
            return 0;
        }
    }
}
