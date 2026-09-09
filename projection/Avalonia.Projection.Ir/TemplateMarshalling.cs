namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of <c>Avalonia.Controls.Templates.IDataTemplate</c>.
/// </summary>
public static class TemplateMarshalling
{
    public const string ManagedTypeName = "Avalonia.Controls.Templates.IDataTemplate";
    public const string InterfaceName = "IAvnDataTemplate";

    public static string QualifiedInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{InterfaceName}";

    public static bool IsDataTemplate(string? managedTypeName) =>
        string.Equals(managedTypeName, ManagedTypeName, StringComparison.Ordinal);
}
