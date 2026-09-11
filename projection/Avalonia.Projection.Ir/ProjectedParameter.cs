namespace Avalonia.Projection.Ir;

public sealed class ProjectedParameter
{
    public required string Name { get; init; }
    public required MarshallingKind Kind { get; init; }
    public required ParameterDirection Direction { get; init; }
    public string? InterfaceName { get; init; }
    public string? ManagedTypeName { get; init; }
    public bool IsNullable { get; init; }

    /// <summary>
    /// For <see cref="MarshallingKind.StringUtf16"/> event fields whose CLR type is not
    /// <c>System.String</c>: the host-side static class that owns the string round-trip.
    /// Mirrors <see cref="ProjectedProperty.StringConverterTypeName"/>.
    /// </summary>
    public string? StringConverterTypeName { get; init; }
}
