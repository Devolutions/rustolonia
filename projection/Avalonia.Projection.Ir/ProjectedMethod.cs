namespace Avalonia.Projection.Ir;

public sealed class ProjectedMethod
{
    public required string Name { get; init; }
    public string? ManagedName { get; init; }
    public required MarshallingKind ReturnKind { get; init; }
    public bool PreserveSig { get; init; }
    public IReadOnlyList<ProjectedParameter> Parameters { get; init; } = [];
}
