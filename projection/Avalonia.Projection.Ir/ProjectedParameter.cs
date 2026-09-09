namespace Avalonia.Projection.Ir;

public sealed class ProjectedParameter
{
    public required string Name { get; init; }
    public required MarshallingKind Kind { get; init; }
    public required ParameterDirection Direction { get; init; }
    public string? InterfaceName { get; init; }
    public string? ManagedTypeName { get; init; }
    public bool IsNullable { get; init; }
}
