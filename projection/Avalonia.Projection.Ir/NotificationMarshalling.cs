namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of
/// <c>Avalonia.Controls.Notifications.INotification</c>.
/// </summary>
public static class NotificationMarshalling
{
    public const string ManagedTypeName = "Avalonia.Controls.Notifications.INotification";
    public const string InterfaceName = "IAvnNotification";

    public static string QualifiedInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{InterfaceName}";

    public static bool IsNotification(string? managedTypeName) =>
        string.Equals(managedTypeName, ManagedTypeName, StringComparison.Ordinal);
}
