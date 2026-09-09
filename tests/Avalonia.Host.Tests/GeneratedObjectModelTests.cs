using System.Reflection;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Interactivity;
using Xunit;

namespace Avalonia.Host.Tests;

public class GeneratedObjectModelTests
{
    [Fact]
    public void Factory_creates_projected_controls_and_forwards_properties()
    {
        var factory = new AvnControlFactory();

        Assert.Equal(0, factory.CreateButton(out var button));
        Assert.Equal(0, factory.CreateTextBlock(out var text));
        Assert.NotNull(button);
        Assert.NotNull(text);

        Assert.Equal(0, text.SetText("Hello from IR"));
        Assert.Equal(0, button.SetContent(text));
        Assert.Equal(0, button.GetContent(out var content));
        Assert.NotNull(content);

        Assert.Equal(0, text.GetObjectId(out var expectedId));
        Assert.Equal(0, content.GetObjectId(out var actualId));
        Assert.Equal(expectedId, actualId);
    }

    [Fact]
    public void Derived_control_implements_generated_base_interfaces()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var button));
        Assert.NotNull(button);

        Assert.IsAssignableFrom<IAvnContentControl>(button);
        Assert.IsAssignableFrom<IAvnControl>(button);
        Assert.IsAssignableFrom<IAvnAvaloniaObject>(button);
    }

    [Fact]
    public void Generated_event_bridge_advises_invokes_and_unadvises()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        var wrapper = Assert.IsType<AvnButton>(projected);
        var value = Assert.IsType<Button>(
            typeof(AvnButton)
                .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var handler = new ClickHandler();

        Assert.Equal(0, wrapper.AdviseClick(handler, out var subscriptionId));
        value.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal(1, handler.CallCount);

        Assert.Equal(0, wrapper.UnadviseClick(subscriptionId));
        value.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal(1, handler.CallCount);
        Assert.True(wrapper.UnadviseClick(subscriptionId) < 0);
    }

    private sealed class ClickHandler : IAvnButtonClickHandler
    {
        public int CallCount { get; private set; }

        public int Invoke()
        {
            CallCount++;
            return 0;
        }
    }
}
