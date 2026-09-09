using System.Reflection;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Input;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU6ThumbDragComTests
{
    [Fact]
    public void Drag_delta_vector_reaches_the_handler()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateThumb(out var projected));
        Assert.NotNull(projected);

        var thumb = Target<Thumb>(projected);
        var handler = new RecordingDragDeltaHandler();
        Assert.Equal(0, ((IAvnThumb)projected).AdviseDragDelta(handler, out var id));

        thumb.RaiseEvent(new VectorEventArgs
        {
            RoutedEvent = Thumb.DragDeltaEvent,
            Vector = new Vector(3, 4),
        });

        Assert.Equal(1, handler.CallCount);
        Assert.Equal(3, handler.Vector.X);
        Assert.Equal(4, handler.Vector.Y);
        Assert.Equal(0, ((IAvnThumb)projected).UnadviseDragDelta(id));
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));

    private sealed class RecordingDragDeltaHandler : IAvnThumbDragDeltaHandler
    {
        public int CallCount { get; private set; }
        public AvnVector Vector { get; private set; }

        public int Invoke(AvnVector vector)
        {
            CallCount++;
            Vector = vector;
            return 0;
        }
    }
}
