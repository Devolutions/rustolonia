using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.Projection.Generator;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

public class GeometryMarshallingEmitterTests
{
    private const string ProbeName =
        "Avalonia.Projection.Generator.Tests.GeometryMarshallingEmitterTests+GeometryProbe";

    public class GeometryProbe
    {
        public Thickness Margin { get; set; }
        public CornerRadius CornerRadius { get; set; }
        public Size ContentSize { get; set; }
        public Point Origin { get; set; }
        public Rect LayoutSlot { get; set; }
        public Color Background { get; set; }
        public Vector Offset { get; set; }
    }

    private static ProjectionIr ProbeIr { get; } = ClrTypeExtractor.Extract(
        [typeof(GeometryProbe)],
        new ProjectionPolicy
        {
            IncludeTypeNames = [ProbeName],
            IncludeMembers = new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal)
            {
                [ProbeName] =
                [
                    nameof(GeometryProbe.Margin),
                    nameof(GeometryProbe.CornerRadius),
                    nameof(GeometryProbe.ContentSize),
                    nameof(GeometryProbe.Origin),
                    nameof(GeometryProbe.LayoutSlot),
                    nameof(GeometryProbe.Background),
                    nameof(GeometryProbe.Offset),
                ],
            },
        });

    [Fact]
    public void Emits_blittable_structs_with_avalonia_conversions()
    {
        var structs = ComSourceEmitter.EmitGeometryStructs(ProbeIr);

        Assert.Contains("[StructLayout(LayoutKind.Sequential)]", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnThickness", structs, StringComparison.Ordinal);
        Assert.Contains(
            "new global::Avalonia.Thickness(Left, Top, Right, Bottom);",
            structs,
            StringComparison.Ordinal);
        Assert.Contains(
            "new global::Avalonia.CornerRadius(TopLeft, TopRight, BottomRight, BottomLeft);",
            structs,
            StringComparison.Ordinal);
        Assert.Contains("public struct AvnCornerRadius", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnSize", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnPoint", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnRect", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnColor", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnVector", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnOptionalRect", structs, StringComparison.Ordinal);
        Assert.Contains("public struct AvnOptionalThickness", structs, StringComparison.Ordinal);
        Assert.Contains("public uint Argb;", structs, StringComparison.Ordinal);
        Assert.Contains(
            "new AvnColor { Argb = value.ToUInt32() };",
            structs,
            StringComparison.Ordinal);
        Assert.Contains(
            "global::Avalonia.Media.Color.FromUInt32(Argb);",
            structs,
            StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_by_value_struct_accessors_on_projected_classes()
    {
        var source = ComSourceEmitter.EmitClass(ProbeIr, ProbeIr.Types.Single());

        Assert.Contains("int GetMargin(out AvnThickness value);", source, StringComparison.Ordinal);
        Assert.Contains("int SetMargin(AvnThickness value);", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnThickness.FromAvalonia(_value.Margin);", source, StringComparison.Ordinal);
        Assert.Contains("_value.Margin = value.ToAvalonia();", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnCornerRadius.FromAvalonia(_value.CornerRadius);", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnSize.FromAvalonia(_value.ContentSize);", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnPoint.FromAvalonia(_value.Origin);", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnRect.FromAvalonia(_value.LayoutSlot);", source, StringComparison.Ordinal);
        Assert.Contains("value = AvnColor.FromAvalonia(_value.Background);", source, StringComparison.Ordinal);
        Assert.Contains("_value.Background = value.ToAvalonia();", source, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_native_struct_typedefs_and_by_value_vtable_slots()
    {
        var header = NativeHeaderEmitter.Emit(ProbeIr);

        Assert.Contains(
            "typedef struct AvnThickness {\n    double left;\n    double top;\n    double right;\n    double bottom;\n} AvnThickness;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnCornerRadius {\n    double top_left;\n    double top_right;\n    double bottom_right;\n    double bottom_left;\n} AvnCornerRadius;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnSize {\n    double width;\n    double height;\n} AvnSize;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnPoint {\n    double x;\n    double y;\n} AvnPoint;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnRect {\n    double x;\n    double y;\n    double width;\n    double height;\n} AvnRect;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnOptionalRect {\n    int32_t has_value;\n    AvnRect value;\n} AvnOptionalRect;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);
        Assert.Contains(
            "typedef struct AvnColor {\n    uint32_t argb;\n} AvnColor;",
            header.ReplaceLineEndings("\n"),
            StringComparison.Ordinal);

        Assert.Contains("*get_margin)(IAvnGeometryProbe* self, AvnThickness* value)", header, StringComparison.Ordinal);
        Assert.Contains("*set_margin)(IAvnGeometryProbe* self, AvnThickness value)", header, StringComparison.Ordinal);
        Assert.Contains("*get_background)(IAvnGeometryProbe* self, AvnColor* value)", header, StringComparison.Ordinal);
        Assert.Contains("*set_background)(IAvnGeometryProbe* self, AvnColor value)", header, StringComparison.Ordinal);
    }

    [Fact]
    public void Kernel_header_publishes_the_geometry_structs_and_live_by_value_slots()
    {
        var header = NativeHeaderEmitter.Emit(KernelIr.Value);

        foreach (var name in new[]
                 {
                     "AvnThickness", "AvnCornerRadius", "AvnSize", "AvnPoint", "AvnRect", "AvnColor",
                     "AvnOptionalRect", "AvnOptionalThickness",
                 })
        {
            Assert.Contains($"typedef struct {name} {{", header, StringComparison.Ordinal);
        }

        // The layout wave allowlisted Control.Margin and Decorator.Padding, so the structs
        // are now carried by real vtable slots rather than only by the echo fixture.
        Assert.Contains(
            "*get_margin)(IAvnControl* self, AvnThickness* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_margin)(IAvnControl* self, AvnThickness value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_padding)(IAvnDecorator* self, AvnThickness value)",
            header,
            StringComparison.Ordinal);

        // Widening IAvnControl republishes it at version 3; IAvnAvaloniaObject projects no
        // members, so its vtable and version are untouched.
        Assert.Contains("#define I_AVN_CONTROL_ABI_VERSION 8", header, StringComparison.Ordinal);
        Assert.Contains(
            "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
            header,
            StringComparison.Ordinal);
    }

    [Theory]
    [InlineData(0d, 0d, 0d, 0d)]
    [InlineData(1.5d, -2.25d, 3d, 4.75d)]
    public void Round_trips_thickness_through_the_abi_struct(
        double left,
        double top,
        double right,
        double bottom)
    {
        var value = new Thickness(left, top, right, bottom);

        var abi = AvnThickness.FromAvalonia(value);

        Assert.Equal(left, abi.Left);
        Assert.Equal(top, abi.Top);
        Assert.Equal(right, abi.Right);
        Assert.Equal(bottom, abi.Bottom);
        Assert.Equal(value, abi.ToAvalonia());
    }

    [Fact]
    public void Round_trips_every_geometry_struct_through_the_abi()
    {
        var cornerRadius = new CornerRadius(1, 2, 3, 4);
        var size = new Size(640, 480);
        var point = new Point(-12.5, 7.25);
        var rect = new Rect(1, 2, 30, 40);
        var color = Color.FromArgb(0x12, 0x34, 0x56, 0x78);

        var abiCornerRadius = AvnCornerRadius.FromAvalonia(cornerRadius);
        Assert.Equal(1d, abiCornerRadius.TopLeft);
        Assert.Equal(2d, abiCornerRadius.TopRight);
        Assert.Equal(3d, abiCornerRadius.BottomRight);
        Assert.Equal(4d, abiCornerRadius.BottomLeft);
        Assert.Equal(cornerRadius, abiCornerRadius.ToAvalonia());

        var abiSize = AvnSize.FromAvalonia(size);
        Assert.Equal(640d, abiSize.Width);
        Assert.Equal(480d, abiSize.Height);
        Assert.Equal(size, abiSize.ToAvalonia());

        var abiPoint = AvnPoint.FromAvalonia(point);
        Assert.Equal(-12.5d, abiPoint.X);
        Assert.Equal(7.25d, abiPoint.Y);
        Assert.Equal(point, abiPoint.ToAvalonia());

        var abiRect = AvnRect.FromAvalonia(rect);
        Assert.Equal(1d, abiRect.X);
        Assert.Equal(2d, abiRect.Y);
        Assert.Equal(30d, abiRect.Width);
        Assert.Equal(40d, abiRect.Height);
        Assert.Equal(rect, abiRect.ToAvalonia());

        var abiColor = AvnColor.FromAvalonia(color);
        Assert.Equal(0x12345678u, abiColor.Argb);
        Assert.Equal(color, abiColor.ToAvalonia());
    }

    [Fact]
    public void Geometry_structs_stay_blittable_and_sequential()
    {
        Assert.Equal(32, Marshal.SizeOf<AvnThickness>());
        Assert.Equal(16, Marshal.SizeOf<AvnSize>());
        Assert.Equal(4, Marshal.SizeOf<AvnColor>());
        Assert.Equal(0, (int)Marshal.OffsetOf<AvnRect>(nameof(AvnRect.X)));
        Assert.Equal(24, (int)Marshal.OffsetOf<AvnRect>(nameof(AvnRect.Height)));
    }

    private static readonly Lazy<ProjectionIr> KernelIr = new(() => ClrTypeExtractor.Extract(
        [
            typeof(AvaloniaObject),
            typeof(StyledElement),
            typeof(Control),
            typeof(Decorator),
        ],
        AvaloniaProjectionProfiles.ObjectModelKernel));
}
