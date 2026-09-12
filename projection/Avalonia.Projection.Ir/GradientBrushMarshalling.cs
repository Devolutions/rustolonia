namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of <c>Avalonia.Media.IGradientBrush</c> and its two supported
/// concrete shapes, <c>ILinearGradientBrush</c> and <c>IRadialGradientBrush</c>.
/// </summary>
/// <remarks>
/// This is an additive capability layered next to <see cref="BrushMarshalling"/>: the existing
/// <c>IAvnBrush</c> (solid colour) interface, IID and vtable are unchanged. A gradient brush
/// projects as a second COM object that implements <c>IAvnBrush</c> (so it still satisfies every
/// brush-typed property slot) alongside the new <c>IAvnGradientBrush</c> capability, queryable
/// with a standard nano-COM QueryInterface. Calling <c>IAvnBrush.GetColor</c> on a real gradient
/// object still fails with <c>AVN_E_NONSOLIDBRUSH</c> — only <c>GetOpacity</c> and the new
/// gradient members are meaningful — so a caller that only understands solid brushes keeps
/// working unchanged. <c>ConicGradientBrush</c>, <c>DrawingBrush</c>, <c>VisualBrush</c> and
/// <c>ImageBrush</c> remain out of scope and keep failing the same way.
///
/// Gradient stops cross as a fixed-capacity inline buffer (<see cref="MaxStops"/> entries) rather
/// than a dynamically sized collection, keeping the ABI shape a single blittable struct; a brush
/// with more stops than that is rejected when it is minted for the ABI.
/// </remarks>
public static class GradientBrushMarshalling
{
    /// <summary>The maximum number of gradient stops a projected gradient brush can carry.</summary>
    public const int MaxStops = 8;

    public const string GradientManagedTypeName = "Avalonia.Media.IGradientBrush";
    public const string LinearManagedTypeName = "Avalonia.Media.ILinearGradientBrush";
    public const string RadialManagedTypeName = "Avalonia.Media.IRadialGradientBrush";
    public const string GradientStopManagedTypeName = "Avalonia.Media.IGradientStop";

    public const string ImmutableLinearManagedTypeName =
        "Avalonia.Media.Immutable.ImmutableLinearGradientBrush";
    public const string ImmutableRadialManagedTypeName =
        "Avalonia.Media.Immutable.ImmutableRadialGradientBrush";
    public const string ImmutableGradientStopManagedTypeName =
        "Avalonia.Media.Immutable.ImmutableGradientStop";

    /// <summary>The unqualified base capability interface name shared by C, C#, and Rust.</summary>
    public const string GradientInterfaceName = "IAvnGradientBrush";

    /// <summary>The unqualified linear-gradient interface name shared by C, C#, and Rust.</summary>
    public const string LinearInterfaceName = "IAvnLinearGradientBrush";

    /// <summary>The unqualified radial-gradient interface name shared by C, C#, and Rust.</summary>
    public const string RadialInterfaceName = "IAvnRadialGradientBrush";

    /// <summary>The factory method that mints a linear gradient brush.</summary>
    public const string CreateLinearFactoryMethodName = "CreateLinearGradientBrush";

    /// <summary>The factory method that mints a radial gradient brush.</summary>
    public const string CreateRadialFactoryMethodName = "CreateRadialGradientBrush";

    public static string QualifiedGradientInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{GradientInterfaceName}";

    public static string QualifiedLinearInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{LinearInterfaceName}";

    public static string QualifiedRadialInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{RadialInterfaceName}";
}
