namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of <c>System.Windows.Input.ICommand</c>.
/// </summary>
public static class CommandMarshalling
{
    public const string ManagedTypeName = "System.Windows.Input.ICommand";
    public const string InterfaceName = "IAvnCommand";
    public const string HandlerInterfaceName = "IAvnCommandCanExecuteChangedHandler";

    public static string QualifiedInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{InterfaceName}";

    public static string QualifiedHandlerInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{HandlerInterfaceName}";

    public static bool IsCommand(string? managedTypeName) =>
        string.Equals(managedTypeName, ManagedTypeName, StringComparison.Ordinal);
}
