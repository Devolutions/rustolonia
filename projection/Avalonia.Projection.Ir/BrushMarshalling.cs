namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of <c>Avalonia.Media.IBrush</c>.
/// </summary>
/// <remarks>
/// A brush crosses nano-COM as a <b>solid colour</b> only: the ABI interface carries a packed
/// <c>AvnColor</c> plus the brush opacity and nothing else. Gradient brushes,
/// <c>DrawingBrush</c> and <c>VisualBrush</c> are deliberately not projected — reading one
/// fails with <c>AVN_E_NONSOLIDBRUSH</c> rather than silently degrading to a nearest colour.
/// The interface is read-only because the managed side hands out immutable brushes; new
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
