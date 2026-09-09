namespace Avalonia.Projection.Ir;

public sealed class ProjectedEnum
{
    public required string Name { get; init; }
    public required string FullName { get; init; }
    public bool IsFlags { get; init; }
    public IReadOnlyList<ProjectedEnumValue> Values { get; init; } = [];
}

public sealed class ProjectedEnumValue
{
    public required string Name { get; init; }
    public required int Value { get; init; }
}
