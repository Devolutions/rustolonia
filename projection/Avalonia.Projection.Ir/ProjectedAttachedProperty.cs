namespace Avalonia.Projection.Ir;

public sealed class ProjectedAttachedProperty
{
    public required string OwnerName { get; init; }
    public required string OwnerManagedFullName { get; init; }
    public required string StaticsInterfaceName { get; init; }
    public required string StaticsInterfaceIid { get; init; }
    public int StaticsInterfaceAbiVersion { get; init; } = 1;
    public required string Name { get; init; }
    public required MarshallingKind Kind { get; init; }
    public required string ManagedTypeName { get; init; }

    /// <summary>
    /// Whether the ABI may carry a null for this attached property. Only string-valued
    /// attached properties can be null today; every scalar one is a value type.
    /// </summary>
    public bool IsNullable { get; init; }

    /// <summary>
    /// The host-side static class that converts this attached property between its CLR type
    /// and the string that crosses the ABI. See
    /// <see cref="ProjectedProperty.StringConverterTypeName"/>.
    /// </summary>
    public string? StringConverterTypeName { get; init; }
}
