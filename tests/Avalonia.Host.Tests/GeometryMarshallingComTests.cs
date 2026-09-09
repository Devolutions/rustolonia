using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Host.Com;
using Avalonia.Media;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Proves the generated geometry ABI structs survive a real CCW/RCW nano-COM round trip.
/// Uses the <see cref="IAvnGeometryEcho"/> ABI fixture, so no published control vtable is widened.
/// </summary>
public unsafe class GeometryMarshallingComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Geometry_structs_round_trip_through_ccw_rcw()
    {
        var echo = new AvnGeometryEcho();
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);

        try
        {
            var rcw = (IAvnGeometryEcho)s_wrappers.GetOrCreateObjectForComInstance(
                unknown,
                CreateObjectFlags.None);

            var thickness = new Thickness(1, 2.5, -3, 4.25);
            Assert.Equal(0, rcw.EchoThickness(AvnThickness.FromAvalonia(thickness), out var abiThickness));
            Assert.Equal(1d, abiThickness.Left);
            Assert.Equal(2.5d, abiThickness.Top);
            Assert.Equal(-3d, abiThickness.Right);
            Assert.Equal(4.25d, abiThickness.Bottom);
            Assert.Equal(thickness, abiThickness.ToAvalonia());

            var cornerRadius = new CornerRadius(5, 6, 7, 8);
            Assert.Equal(
                0,
                rcw.EchoCornerRadius(AvnCornerRadius.FromAvalonia(cornerRadius), out var abiCornerRadius));
            Assert.Equal(cornerRadius, abiCornerRadius.ToAvalonia());

            var size = new Size(640, 480);
            Assert.Equal(0, rcw.EchoSize(AvnSize.FromAvalonia(size), out var abiSize));
            Assert.Equal(size, abiSize.ToAvalonia());

            var point = new Point(-12.5, 7.25);
            Assert.Equal(0, rcw.EchoPoint(AvnPoint.FromAvalonia(point), out var abiPoint));
            Assert.Equal(point, abiPoint.ToAvalonia());

            var rect = new Rect(1, 2, 30, 40);
            Assert.Equal(0, rcw.EchoRect(AvnRect.FromAvalonia(rect), out var abiRect));
            Assert.Equal(rect, abiRect.ToAvalonia());

            var color = Color.FromArgb(0x12, 0x34, 0x56, 0x78);
            Assert.Equal(0, rcw.EchoColor(AvnColor.FromAvalonia(color), out var abiColor));
            Assert.Equal(0x12345678u, abiColor.Argb);
            Assert.Equal(color, abiColor.ToAvalonia());
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    [Fact]
    public void Geometry_structs_are_blittable_and_sequential()
    {
        Assert.Equal(32, Marshal.SizeOf<AvnThickness>());
        Assert.Equal(32, Marshal.SizeOf<AvnCornerRadius>());
        Assert.Equal(16, Marshal.SizeOf<AvnSize>());
        Assert.Equal(16, Marshal.SizeOf<AvnPoint>());
        Assert.Equal(16, Marshal.SizeOf<AvnVector>());
        Assert.Equal(32, Marshal.SizeOf<AvnRect>());
        Assert.Equal(4, Marshal.SizeOf<AvnColor>());

        Assert.Equal(0, (int)Marshal.OffsetOf<AvnThickness>(nameof(AvnThickness.Left)));
        Assert.Equal(8, (int)Marshal.OffsetOf<AvnThickness>(nameof(AvnThickness.Top)));
        Assert.Equal(16, (int)Marshal.OffsetOf<AvnThickness>(nameof(AvnThickness.Right)));
        Assert.Equal(24, (int)Marshal.OffsetOf<AvnThickness>(nameof(AvnThickness.Bottom)));

        Assert.Equal(0, (int)Marshal.OffsetOf<AvnCornerRadius>(nameof(AvnCornerRadius.TopLeft)));
        Assert.Equal(24, (int)Marshal.OffsetOf<AvnCornerRadius>(nameof(AvnCornerRadius.BottomLeft)));
        Assert.Equal(8, (int)Marshal.OffsetOf<AvnSize>(nameof(AvnSize.Height)));
        Assert.Equal(8, (int)Marshal.OffsetOf<AvnPoint>(nameof(AvnPoint.Y)));
        Assert.Equal(24, (int)Marshal.OffsetOf<AvnRect>(nameof(AvnRect.Height)));
        Assert.Equal(0, (int)Marshal.OffsetOf<AvnColor>(nameof(AvnColor.Argb)));
    }
}
