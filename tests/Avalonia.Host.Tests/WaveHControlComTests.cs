using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls.Shapes;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveHControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Rectangle_fill_and_radii_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateSolidColorBrush(AvnColor.FromAvalonia(Color.FromRgb(0x00, 0x7A, 0xCC)), 1, out var brush));
        Assert.Equal(0, factory.CreateRectangle(out var projected));
        Assert.NotNull(projected);
        Assert.NotNull(brush);

        Through<IAvnRectangle>(projected, rectangle =>
        {
            Assert.Equal(0, rectangle.SetFill(brush));
            Assert.Equal(0, rectangle.SetStrokeThickness(2));
            Assert.Equal(0, rectangle.SetRadiusX(4));
            Assert.Equal(0, rectangle.SetRadiusY(6));
        });

        var value = Target<Rectangle>(projected);
        Assert.NotNull(value.Fill);
        Assert.Equal(2, value.StrokeThickness);
        Assert.Equal(4, value.RadiusX);
        Assert.Equal(6, value.RadiusY);
    }

    [Fact]
    public void Line_points_and_path_data_cross()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateLine(out var projectedLine));
        Assert.Equal(0, factory.CreatePath(out var projectedPath));
        Assert.NotNull(projectedLine);
        Assert.NotNull(projectedPath);

        Through<IAvnLine>(projectedLine, line =>
        {
            Assert.Equal(0, line.SetStartPoint(new AvnPoint { X = 1, Y = 2 }));
            Assert.Equal(0, line.SetEndPoint(new AvnPoint { X = 10, Y = 20 }));
        });
        Through<IAvnPath>(projectedPath, path =>
        {
            Assert.Equal(0, path.SetData("M0,0 L10,10"));
            Assert.Equal(0, path.GetData(out var data));
            Assert.False(string.IsNullOrEmpty(data));
        });

        var lineValue = Target<Line>(projectedLine);
        Assert.Equal(1, lineValue.StartPoint.X);
        Assert.Equal(20, lineValue.EndPoint.Y);
        Assert.NotNull(Target<Path>(projectedPath).Data);
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
