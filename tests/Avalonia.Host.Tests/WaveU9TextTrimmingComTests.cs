using System.Reflection;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU9TextTrimmingComTests
{
    [Fact]
    public void Text_trimming_name_reaches_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBlock(out var projected));
        Assert.NotNull(projected);

        Through<IAvnTextBlock>(projected, block =>
        {
            Assert.Equal(0, block.SetTextTrimming("CharacterEllipsis"));
            Assert.Equal(0, block.GetTextTrimming(out var value));
            Assert.Equal("CharacterEllipsis", value);
        });
        Assert.Equal(TextTrimming.CharacterEllipsis, Target<TextBlock>(projected).TextTrimming);
    }

    private static readonly System.Runtime.InteropServices.Marshalling.StrategyBasedComWrappers s_wrappers = new();

    private static void Through<T>(object wrapper, System.Action<T> body) where T : class
    {
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(
            wrapper, System.Runtime.InteropServices.CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);
        try
        {
            body((T)s_wrappers.GetOrCreateObjectForComInstance(
                unknown, System.Runtime.InteropServices.CreateObjectFlags.None));
        }
        finally
        {
            System.Runtime.InteropServices.Marshal.Release(unknown);
        }
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
