namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of <c>Avalonia.Media.IBrush</c>.
/// </summary>
/// <remarks>
/// <c>IAvnBrush</c> itself still carries a <b>solid colour</b> only: a packed <c>AvnColor</c>
/// plus the brush opacity and nothing else, unchanged since it first shipped. Linear and radial
/// gradient brushes are projected as an additive sibling capability (see
/// <see cref="GradientBrushMarshalling"/>) that a gradient brush object also implements;
/// <c>DrawingBrush</c>, <c>VisualBrush</c>, <c>ImageBrush</c> and conic gradients remain out of
/// scope — reading any of those, or calling <c>IAvnBrush.GetColor</c> on a real gradient object,
/// fails with <c>AVN_E_NONSOLIDBRUSH</c> rather than silently degrading to a nearest colour.
/// The interface is read-only because the managed side hands out immutable brushes; new solid
/// brushes are minted through <c>IAvnControlFactory.CreateSolidColorBrush</c>.
/// </remarks>
public static class BrushMarshalling
{
    /// <summary>The CLR type that maps onto <see cref="MarshallingKind.Brush"/>.</summary>
    public const string ManagedTypeName = "Avalonia.Media.IBrush";

    /// <summary>The CLR interface a projected brush must implement to be readable.</summary>
    public const string SolidManagedTypeName = "Avalonia.Media.ISolidColorBrush";

    /// <summary>The CLR type constructed when a brush crosses the ABI inbound.</summary>
    public const string ImmutableSolidManagedTypeName =
        "Avalonia.Media.Immutable.ImmutableSolidColorBrush";

    /// <summary>The unqualified ABI interface name shared by C, C#, and Rust.</summary>
    public const string InterfaceName = "IAvnBrush";

    /// <summary>The factory method that mints a solid colour brush.</summary>
    public const string FactoryMethodName = "CreateSolidColorBrush";

    public static string QualifiedInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{InterfaceName}";

    public static bool IsBrush(string? managedTypeName) =>
        string.Equals(managedTypeName, ManagedTypeName, StringComparison.Ordinal);
}
