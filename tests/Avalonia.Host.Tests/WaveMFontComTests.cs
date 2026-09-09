using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveMFontComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Templated_control_and_text_block_fonts_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projectedButton));
        Assert.Equal(0, factory.CreateTextBlock(out var projectedText));
        Assert.NotNull(projectedButton);
        Assert.NotNull(projectedText);

        Through<IAvnButton>(projectedButton, button =>
        {
            Assert.Equal(0, button.SetFontFamily("Courier New"));
            Assert.Equal(0, button.SetFontStyle((int)FontStyle.Italic));
            Assert.Equal(0, button.SetFontWeight((int)FontWeight.Bold));
            Assert.Equal(0, button.SetLetterSpacing(1.5));
        });
        Through<IAvnTextBlock>(projectedText, text =>
        {
            Assert.Equal(0, text.SetFontFamily("Segoe UI"));
            Assert.Equal(0, text.SetFontStyle((int)FontStyle.Oblique));
            Assert.Equal(0, text.SetMaxLines(3));
            Assert.Equal(0, text.SetTextWrapping((int)TextWrapping.Wrap));
            Assert.Equal(0, text.SetLineSpacing(4));
        });

        var button = Target<Button>(projectedButton);
        Assert.Equal("Courier New", button.FontFamily.ToString());
        Assert.Equal(FontStyle.Italic, button.FontStyle);
        Assert.Equal(FontWeight.Bold, button.FontWeight);
        Assert.Equal(1.5, button.LetterSpacing);

        var value = Target<TextBlock>(projectedText);
        Assert.Equal("Segoe UI", value.FontFamily.ToString());
        Assert.Equal(FontStyle.Oblique, value.FontStyle);
        Assert.Equal(3, value.MaxLines);
        Assert.Equal(TextWrapping.Wrap, value.TextWrapping);
        Assert.Equal(4, value.LineSpacing);
    }

    private static void Through<T>(object wrapper, Action<T> body) where T : class
    {
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(wrapper, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);
        try
        {
            body((T)s_wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None));
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
