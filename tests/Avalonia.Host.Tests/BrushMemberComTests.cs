using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.Media.Immutable;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the solid brush ABI and the chrome members published on the widened
/// <c>IAvnBorder</c>, <c>IAvnPanel</c>, <c>IAvnTemplatedControl</c> and <c>IAvnTextBlock</c>
/// vtables. Every assertion goes through a real CCW/RCW round trip, so an <c>IAvnBrush</c>
/// crosses the generated marshalling stubs as an interface pointer rather than being read
/// straight off the managed wrapper.
/// </summary>
public unsafe class BrushMemberComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Factory_mints_a_solid_brush_that_round_trips_colour_and_opacity()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(
            0,
            factory.CreateSolidColorBrush(
                AvnColor.FromAvalonia(Color.FromArgb(0xFF, 0x33, 0x66, 0x99)),
                0.5,
                out var projected));
        Assert.NotNull(projected);

        Through<IAvnBrush>(projected, brush =>
        {
            Assert.Equal(0, brush.GetColor(out var color));
            Assert.Equal(Color.FromArgb(0xFF, 0x33, 0x66, 0x99), color.ToAvalonia());

            Assert.Equal(0, brush.GetOpacity(out var opacity));
            Assert.Equal(0.5, opacity);
        });
    }

    [Fact]
    public void Border_chrome_members_round_trip_and_reach_the_avalonia_object()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);
        var border = Target<Border>(projected);
        var background = SolidBrush(factory, Color.FromArgb(0xFF, 0x10, 0x20, 0x30), 1);
        var borderBrush = SolidBrush(factory, Color.FromArgb(0x80, 0xAA, 0xBB, 0xCC), 0.25);

        Through<IAvnBorder>(projected, value =>
        {
            Assert.Equal(0, value.GetBackground(out var initial));
            Assert.Null(initial);

            Assert.Equal(0, value.SetBackground(background));
            Assert.Equal(0, value.GetBackground(out var readBack));
            Assert.NotNull(readBack);
            Assert.Equal(0, readBack.GetColor(out var color));
            Assert.Equal(Color.FromArgb(0xFF, 0x10, 0x20, 0x30), color.ToAvalonia());

            Assert.Equal(0, value.SetBorderBrush(borderBrush));
            Assert.Equal(0, value.GetBorderBrush(out var readBorder));
            Assert.NotNull(readBorder);
            Assert.Equal(0, readBorder.GetColor(out var borderColor));
            Assert.Equal(0, readBorder.GetOpacity(out var borderOpacity));
            Assert.Equal(Color.FromArgb(0x80, 0xAA, 0xBB, 0xCC), borderColor.ToAvalonia());
            Assert.Equal(0.25, borderOpacity);

            Assert.Equal(0, value.SetBorderThickness(AvnThickness.FromAvalonia(new Thickness(1, 2, 3, 4))));
            Assert.Equal(0, value.GetBorderThickness(out var thickness));
            Assert.Equal(new Thickness(1, 2, 3, 4), thickness.ToAvalonia());

            Assert.Equal(0, value.SetCornerRadius(AvnCornerRadius.FromAvalonia(new CornerRadius(6))));
            Assert.Equal(0, value.GetCornerRadius(out var cornerRadius));
            Assert.Equal(new CornerRadius(6), cornerRadius.ToAvalonia());
        });

        // The projection must have written through to the real control, not to wrapper state.
        var solid = Assert.IsAssignableFrom<ISolidColorBrush>(border.Background);
        Assert.Equal(Color.FromArgb(0xFF, 0x10, 0x20, 0x30), solid.Color);
        Assert.Equal(1d, solid.Opacity);
        var stroke = Assert.IsAssignableFrom<ISolidColorBrush>(border.BorderBrush);
        Assert.Equal(Color.FromArgb(0x80, 0xAA, 0xBB, 0xCC), stroke.Color);
        Assert.Equal(0.25, stroke.Opacity);
        Assert.Equal(new Thickness(1, 2, 3, 4), border.BorderThickness);
        Assert.Equal(new CornerRadius(6), border.CornerRadius);
    }

    [Fact]
    public void A_null_brush_clears_the_avalonia_property()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);
        var border = Target<Border>(projected);

        Through<IAvnBorder>(projected, value =>
        {
            Assert.Equal(0, value.SetBackground(SolidBrush(factory, Colors.Red, 1)));
            Assert.Equal(0, value.SetBackground(null));
            Assert.Equal(0, value.GetBackground(out var cleared));
            Assert.Null(cleared);
        });

        Assert.Null(border.Background);
    }

    [Fact]
    public void A_non_solid_brush_fails_explicitly_instead_of_picking_a_nearest_colour()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);
        Target<Border>(projected).Background = new LinearGradientBrush
        {
            GradientStops =
            {
                new GradientStop(Colors.Red, 0),
                new GradientStop(Colors.Blue, 1),
            },
        };

        Through<IAvnBorder>(projected, value =>
            Assert.Equal(HResults.AVN_E_NONSOLIDBRUSH, value.GetBackground(out _)));
    }

    [Fact]
    public void Panel_background_round_trips()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateStackPanel(out var projected));
        Assert.NotNull(projected);

        Through<IAvnStackPanel>(projected, value =>
        {
            Assert.Equal(0, value.SetBackground(SolidBrush(factory, Colors.Green, 1)));
            Assert.Equal(0, value.GetBackground(out var background));
            Assert.NotNull(background);
            Assert.Equal(0, background.GetColor(out var color));
            Assert.Equal(Colors.Green, color.ToAvalonia());
        });

        Assert.Equal(
            Colors.Green,
            Assert.IsAssignableFrom<ISolidColorBrush>(Target<StackPanel>(projected).Background).Color);
    }

    [Fact]
    public void Templated_control_chrome_members_round_trip()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);
        var button = Target<Button>(projected);

        Through<IAvnButton>(projected, value =>
        {
            Assert.Equal(0, value.SetBackground(SolidBrush(factory, Colors.Navy, 1)));
            Assert.Equal(0, value.SetForeground(SolidBrush(factory, Colors.White, 1)));
            Assert.Equal(0, value.SetBorderBrush(SolidBrush(factory, Colors.Silver, 1)));
            Assert.Equal(0, value.SetBorderThickness(AvnThickness.FromAvalonia(new Thickness(2))));
            Assert.Equal(0, value.SetCornerRadius(AvnCornerRadius.FromAvalonia(new CornerRadius(4))));
            Assert.Equal(0, value.SetFontSize(18));

            Assert.Equal(0, value.GetForeground(out var foreground));
            Assert.NotNull(foreground);
            Assert.Equal(0, foreground.GetColor(out var color));
            Assert.Equal(Colors.White, color.ToAvalonia());
            Assert.Equal(0, value.GetFontSize(out var fontSize));
            Assert.Equal(18d, fontSize);
        });

        Assert.Equal(
            Colors.Navy,
            Assert.IsAssignableFrom<ISolidColorBrush>(button.Background).Color);
        Assert.Equal(
            Colors.Silver,
            Assert.IsAssignableFrom<ISolidColorBrush>(button.BorderBrush).Color);
        Assert.Equal(new Thickness(2), button.BorderThickness);
        Assert.Equal(new CornerRadius(4), button.CornerRadius);
        Assert.Equal(18d, button.FontSize);
    }

    [Fact]
    public void Text_block_text_members_round_trip()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBlock(out var projected));
        Assert.NotNull(projected);
        var textBlock = Target<TextBlock>(projected);

        Through<IAvnTextBlock>(projected, value =>
        {
            Assert.Equal(0, value.SetFontSize(21.5));
            Assert.Equal(0, value.SetFontWeight((int)FontWeight.Bold));
            Assert.Equal(0, value.SetTextAlignment((int)TextAlignment.Center));
            Assert.Equal(0, value.SetPadding(AvnThickness.FromAvalonia(new Thickness(3, 5))));
            Assert.Equal(0, value.SetForeground(SolidBrush(factory, Colors.Teal, 0.8)));

            Assert.Equal(0, value.GetFontSize(out var fontSize));
            Assert.Equal(21.5, fontSize);
            Assert.Equal(0, value.GetFontWeight(out var fontWeight));
            Assert.Equal((int)FontWeight.Bold, fontWeight);
            Assert.Equal(0, value.GetTextAlignment(out var alignment));
            Assert.Equal((int)TextAlignment.Center, alignment);
            Assert.Equal(0, value.GetPadding(out var padding));
            Assert.Equal(new Thickness(3, 5), padding.ToAvalonia());
            Assert.Equal(0, value.GetForeground(out var foreground));
            Assert.NotNull(foreground);
            Assert.Equal(0, foreground.GetOpacity(out var opacity));
            Assert.Equal(0.8, opacity);
        });

        Assert.Equal(21.5, textBlock.FontSize);
        Assert.Equal(FontWeight.Bold, textBlock.FontWeight);
        Assert.Equal(TextAlignment.Center, textBlock.TextAlignment);
        Assert.Equal(new Thickness(3, 5), textBlock.Padding);
        Assert.Equal(
            Colors.Teal,
            Assert.IsAssignableFrom<ISolidColorBrush>(textBlock.Foreground).Color);
    }

    [Fact]
    public void Brush_conversion_helpers_are_lossless_in_both_directions()
    {
        var source = new ImmutableSolidColorBrush(Color.FromArgb(0x40, 0x01, 0x02, 0x03), 0.75);

        var projected = AvnBrush.FromBrush(source);
        Assert.NotNull(projected);
        var materialised = Assert.IsAssignableFrom<ISolidColorBrush>(AvnBrush.ToBrush(projected));

        Assert.Equal(source.Color, materialised.Color);
        Assert.Equal(source.Opacity, materialised.Opacity);
        Assert.Null(AvnBrush.FromBrush(null));
        Assert.Null(AvnBrush.ToBrush(null));
    }

    private static IAvnBrush SolidBrush(AvnControlFactory factory, Color color, double opacity)
    {
        Assert.Equal(
            0,
            factory.CreateSolidColorBrush(AvnColor.FromAvalonia(color), opacity, out var brush));
        return Assert.IsType<AvnBrush>(brush);
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
