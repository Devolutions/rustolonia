using System.Reflection;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU8LabelTargetComTests
{
    [Fact]
    public void Label_target_reaches_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateLabel(out var projectedLabel));
        Assert.Equal(0, factory.CreateTextBox(out var projectedBox));
        Assert.NotNull(projectedLabel);
        Assert.NotNull(projectedBox);

        Assert.Equal(0, ((IAvnLabel)projectedLabel).SetTarget((IAvnControl)projectedBox));
        Assert.Same(Target<TextBox>(projectedBox), Target<Label>(projectedLabel).Target);
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
