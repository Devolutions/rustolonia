using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Media;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class GeometryMarshallingTests
{
    private const string ProbeName = "Avalonia.Projection.Ir.Tests.GeometryMarshallingTests+GeometryProbe";

    public class GeometryProbe
    {
        public Thickness Margin { get; set; }
        public CornerRadius CornerRadius { get; set; }
        public Size ContentSize { get; set; }
        public Point Origin { get; set; }
        public Rect LayoutSlot { get; set; }
        public Color Background { get; set; }
        public Vector Offset { get; set; }
        public Thickness? OptionalMargin { get; set; }
    }

    private static ProjectionPolicy Policy { get; } = new()
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
                nameof(GeometryProbe.OptionalMargin),
            ],
        },
    };

    [Fact]
    public void Maps_every_geometry_clr_type_to_its_marshalling_kind()
    {
        var ir = ClrTypeExtractor.Extract([typeof(GeometryProbe)], Policy);
        var probe = Assert.Single(ir.Types);

        AssertProperty(probe, nameof(GeometryProbe.Margin), MarshallingKind.Thickness, "Avalonia.Thickness");
        AssertProperty(probe, nameof(GeometryProbe.CornerRadius), MarshallingKind.CornerRadius, "Avalonia.CornerRadius");
        AssertProperty(probe, nameof(GeometryProbe.ContentSize), MarshallingKind.Size, "Avalonia.Size");
        AssertProperty(probe, nameof(GeometryProbe.Origin), MarshallingKind.Point, "Avalonia.Point");
        AssertProperty(probe, nameof(GeometryProbe.LayoutSlot), MarshallingKind.Rect, "Avalonia.Rect");
        AssertProperty(probe, nameof(GeometryProbe.Background), MarshallingKind.Color, "Avalonia.Media.Color");
        AssertProperty(probe, nameof(GeometryProbe.Offset), MarshallingKind.Vector, "Avalonia.Vector");
    }

    [Fact]
    public void Maps_nullable_geometry_as_the_same_kind_with_is_nullable()
    {
        var ir = ClrTypeExtractor.Extract([typeof(GeometryProbe)], Policy);
        var property = ir.Types.Single().Properties.Single(p => p.Name == nameof(GeometryProbe.OptionalMargin));
        Assert.Equal(MarshallingKind.Thickness, property.Kind);
        Assert.True(property.IsNullable);
        Assert.DoesNotContain(ir.Skipped, entry => entry.Member == nameof(GeometryProbe.OptionalMargin));
    }

    [Fact]
    public void Describes_every_geometry_struct_as_a_blittable_sequential_shape()
    {
        Assert.Equal(7, GeometryMarshalling.All.Count);
        Assert.All(GeometryMarshalling.All, geometry => Assert.NotEmpty(geometry.Fields));
        Assert.All(
            GeometryMarshalling.All,
            geometry => Assert.Equal("Avn" + geometry.Kind, geometry.AbiName));

        AssertShape(MarshallingKind.Thickness, "Avalonia.Thickness", ["Left", "Top", "Right", "Bottom"]);
        AssertShape(
            MarshallingKind.CornerRadius,
            "Avalonia.CornerRadius",
            ["TopLeft", "TopRight", "BottomRight", "BottomLeft"]);
        AssertShape(MarshallingKind.Size, "Avalonia.Size", ["Width", "Height"]);
        AssertShape(MarshallingKind.Point, "Avalonia.Point", ["X", "Y"]);
        AssertShape(MarshallingKind.Vector, "Avalonia.Vector", ["X", "Y"]);
        AssertShape(MarshallingKind.Rect, "Avalonia.Rect", ["X", "Y", "Width", "Height"]);

        Assert.True(GeometryMarshalling.TryGet(MarshallingKind.Color, out var color));
        Assert.Equal(GeometryConversion.PackedColor, color.Conversion);
        var argb = Assert.Single(color.Fields);
        Assert.Equal("Argb", argb.Name);
        Assert.Equal("argb", argb.NativeName);
        Assert.Equal(GeometryFieldKind.UInt32, argb.Kind);
    }

    [Fact]
    public void Leaves_previously_published_marshalling_kinds_untouched()
    {
        Assert.Equal(0, (int)MarshallingKind.Unsupported);
        Assert.Equal(10, (int)MarshallingKind.ComCollection);
        Assert.Equal(11, (int)MarshallingKind.Thickness);
        Assert.Equal(16, (int)MarshallingKind.Color);

        // Brush was appended after the geometry kinds rather than grouped with the interface
        // kinds, so nothing that shipped earlier moved.
        Assert.Equal(17, (int)MarshallingKind.Brush);
        Assert.Equal(18, (int)MarshallingKind.Vector);
        Assert.Equal(19, (int)MarshallingKind.CharUtf16);
        Assert.Equal(20, (int)MarshallingKind.Command);
    }

    [Fact]
    public void Maps_geometry_structs_in_hand_written_com_interfaces()
    {
        var ir = ComInterfaceExtractor.Extract(
            typeof(Host.Com.IAvnGeometryEcho).Assembly,
            new ProjectionPolicy { IncludeTypeNames = ["Avalonia.Host.Com.IAvnGeometryEcho"] });

        var echo = Assert.Single(ir.Types, type => type.Name == "IAvnGeometryEcho");
        Assert.Empty(ir.Skipped);
        AssertEchoMethod(echo, "EchoThickness", MarshallingKind.Thickness);
        AssertEchoMethod(echo, "EchoCornerRadius", MarshallingKind.CornerRadius);
        AssertEchoMethod(echo, "EchoSize", MarshallingKind.Size);
        AssertEchoMethod(echo, "EchoPoint", MarshallingKind.Point);
        AssertEchoMethod(echo, "EchoRect", MarshallingKind.Rect);
        AssertEchoMethod(echo, "EchoColor", MarshallingKind.Color);
    }

    private static void AssertEchoMethod(ProjectedType type, string name, MarshallingKind kind)
    {
        var method = Assert.Single(type.Methods, candidate => candidate.Name == name);
        Assert.All(method.Parameters, parameter => Assert.Equal(kind, parameter.Kind));
        Assert.Equal(2, method.Parameters.Count);
        Assert.Equal(ParameterDirection.Out, method.Parameters[1].Direction);
        Assert.True(GeometryMarshalling.TryGet(kind, out var geometry));
        Assert.True(GeometryMarshalling.TryGetByAbiName(geometry.AbiName, out var byAbiName));
        Assert.Equal(kind, byAbiName.Kind);
    }

    private static void AssertShape(
        MarshallingKind kind,
        string managedTypeName,
        string[] fieldNames)
    {
        Assert.True(GeometryMarshalling.TryGet(kind, out var geometry));
        Assert.Equal(managedTypeName, geometry.ManagedTypeName);
        Assert.Equal(GeometryConversion.Components, geometry.Conversion);
        Assert.Equal(fieldNames, geometry.Fields.Select(field => field.Name).ToArray());
        Assert.All(geometry.Fields, field => Assert.Equal(GeometryFieldKind.Double, field.Kind));
        Assert.True(GeometryMarshalling.TryGetByManagedTypeName(managedTypeName, out var byName));
        Assert.Equal(kind, byName.Kind);
    }

    private static void AssertProperty(
        ProjectedType type,
        string name,
        MarshallingKind kind,
        string managedTypeName)
    {
        var property = Assert.Single(type.Properties, candidate => candidate.Name == name);
        Assert.Equal(kind, property.Kind);
        Assert.Equal(managedTypeName, property.ManagedTypeName);
        Assert.Null(property.InterfaceName);
        Assert.True(property.CanRead);
        Assert.True(property.CanWrite);
    }
}
