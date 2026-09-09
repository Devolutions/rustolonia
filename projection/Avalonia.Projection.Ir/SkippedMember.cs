namespace Avalonia.Projection.Ir;

public sealed class SkippedMember
{
    public required string Owner { get; init; }
    public required string Member { get; init; }
    public required string Reason { get; init; }
}
