namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of the custom popup placement callback, which
/// receives a mutable placement record and positions the popup by writing its
/// fields back.
/// </summary>
public static class PopupPlacementMarshalling
{
    public const string ManagedTypeName =
        "Avalonia.Controls.Primitives.PopupPositioning.CustomPopupPlacementCallback";
    public const string InterfaceName = "IAvnPopupPlacementCallback";

    public static string QualifiedInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{InterfaceName}";

    public static bool IsPopupPlacementCallback(Type? type) =>
        type?.FullName == ManagedTypeName;
}
