using System.Diagnostics.CodeAnalysis;

namespace Avalonia.Projection.Ir;

public enum GeometryFieldKind
{
    Double,
    UInt32,
}

public enum GeometryConversion
{
    /// <summary>ABI fields map one-to-one onto managed properties and the managed constructor.</summary>
    Components,

    /// <summary>A packed ARGB integer matching <c>Avalonia.Media.Color.ToUInt32()</c>.</summary>
    PackedColor,
}

public sealed record GeometryField(string Name, string NativeName, GeometryFieldKind Kind);

public sealed record GeometryStruct(
    MarshallingKind Kind,
    string ManagedTypeName,
    string AbiName,
    GeometryConversion Conversion,
    IReadOnlyList<GeometryField> Fields)
{
    /// <summary>
    /// Blittable wrapper that carries <see cref="AbiName"/> plus a has-value flag so a
    /// nullable CLR geometry value can cross without a sentinel inside the struct itself.
    /// </summary>
    public string OptionalAbiName => "AvnOptional" + Kind;
}

/// <summary>
/// Blittable Avalonia value types that cross the nano-COM ABI by value instead of as COM objects.
/// Every struct is sequential and blittable so the C, C#, and Rust declarations share one layout.
/// </summary>
public static class GeometryMarshalling
{
    public static IReadOnlyList<GeometryStruct> All { get; } =
    [
        new(
            MarshallingKind.Thickness,
            "Avalonia.Thickness",
            "AvnThickness",
            GeometryConversion.Components,
            [Double("Left"), Double("Top"), Double("Right"), Double("Bottom")]),
        new(
            MarshallingKind.CornerRadius,
            "Avalonia.CornerRadius",
            "AvnCornerRadius",
            GeometryConversion.Components,
            [Double("TopLeft"), Double("TopRight"), Double("BottomRight"), Double("BottomLeft")]),
        new(
            MarshallingKind.Size,
            "Avalonia.Size",
            "AvnSize",
            GeometryConversion.Components,
            [Double("Width"), Double("Height")]),
        new(
            MarshallingKind.Point,
            "Avalonia.Point",
            "AvnPoint",
            GeometryConversion.Components,
            [Double("X"), Double("Y")]),
        new(
            MarshallingKind.Rect,
            "Avalonia.Rect",
            "AvnRect",
            GeometryConversion.Components,
            [Double("X"), Double("Y"), Double("Width"), Double("Height")]),
        new(
            MarshallingKind.Color,
            "Avalonia.Media.Color",
            "AvnColor",
            GeometryConversion.PackedColor,
            [new GeometryField("Argb", "argb", GeometryFieldKind.UInt32)]),
        new(
            MarshallingKind.Vector,
            "Avalonia.Vector",
            "AvnVector",
            GeometryConversion.Components,
            [Double("X"), Double("Y")]),
    ];

    public static bool IsGeometry(MarshallingKind kind) =>
        All.Any(value => value.Kind == kind);

    public static bool TryGet(MarshallingKind kind, [NotNullWhen(true)] out GeometryStruct? value)
    {
        value = All.FirstOrDefault(candidate => candidate.Kind == kind);
        return value is not null;
    }

    public static bool TryGetByManagedTypeName(
        string? managedTypeName,
        [NotNullWhen(true)] out GeometryStruct? value)
    {
        value = managedTypeName is null
            ? null
            : All.FirstOrDefault(candidate =>
                string.Equals(candidate.ManagedTypeName, managedTypeName, StringComparison.Ordinal));
        return value is not null;
    }

    public static bool TryGetByAbiName(
        string? abiName,
        [NotNullWhen(true)] out GeometryStruct? value)
    {
        value = abiName is null
            ? null
            : All.FirstOrDefault(candidate =>
                string.Equals(candidate.AbiName, abiName, StringComparison.Ordinal));
        return value is not null;
    }

    private static GeometryField Double(string name) =>
        new(name, Snake(name), GeometryFieldKind.Double);

    private static string Snake(string value)
    {
        var builder = new System.Text.StringBuilder();
        for (var index = 0; index < value.Length; index++)
        {
            if (char.IsUpper(value[index]) && index > 0)
                builder.Append('_');
            builder.Append(char.ToLowerInvariant(value[index]));
        }
        return builder.ToString();
    }
}
