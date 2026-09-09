using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU22ShapeScalarsComTests
{
    [Fact]
    public void Polygon_points_cross_as_a_text_list()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePolygon(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnPolygon>(projected);

        Assert.Equal(0, wrapper.SetPoints("10,20 30,40"));
        var value = Assert.IsType<Polygon>(
            typeof(AvnPolygon)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Equal(2, value.Points.Count);
        Assert.Equal(10, value.Points[0].X);
        Assert.Equal(40, value.Points[1].Y);

        Assert.Equal(0, wrapper.GetPoints(out var read));
        Assert.Equal("10,20 30,40", read);
    }

    [Fact]
    public void Shape_stroke_dash_array_crosses_as_a_text_list()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateRectangle(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnRectangle>(projected);

        Assert.Equal(0, wrapper.SetStrokeDashArray("2,3"));
        var value = Assert.IsType<Rectangle>(
            typeof(AvnRectangle)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.Equal(2, value.StrokeDashArray!.Count);
        Assert.Equal(0, wrapper.GetStrokeDashArray(out var read));
        Assert.Equal("2,3", read);

        Assert.Equal(0, wrapper.SetStrokeDashArray(null));
        Assert.Equal(0, wrapper.GetStrokeDashArray(out var cleared));
        Assert.Null(cleared);
    }

    [Fact]
    public void Text_block_font_features_cross_as_a_text_list()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBlock(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnTextBlock>(projected);

        Assert.Equal(0, wrapper.SetFontFeatures("ss01,liga"));
        Assert.Equal(0, wrapper.GetFontFeatures(out var read));
        Assert.NotNull(read);
    }
}
