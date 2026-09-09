namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of AutoCompleteBox's filter predicates.
/// </summary>
public static class FilterMarshalling
{
    public const string ItemFilterManagedTypeName =
        "Avalonia.Controls.AutoCompleteFilterPredicate`1";
    public const string TextFilterManagedTypeName =
        "Avalonia.Controls.AutoCompleteFilterPredicate`1";

    public const string ItemFilterInterfaceName = "IAvnItemFilter";
    public const string TextFilterInterfaceName = "IAvnTextFilter";

    public static string QualifiedItemFilterInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{ItemFilterInterfaceName}";

    public static string QualifiedTextFilterInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{TextFilterInterfaceName}";

    public static bool IsItemFilter(Type? type) =>
        type is not null &&
        type.IsGenericType &&
        type.GetGenericTypeDefinition().FullName == ItemFilterManagedTypeName &&
        type.GetGenericArguments()[0] == typeof(object);

    public static bool IsTextFilter(Type? type) =>
        type is not null &&
        type.IsGenericType &&
        type.GetGenericTypeDefinition().FullName == TextFilterManagedTypeName &&
        type.GetGenericArguments()[0] == typeof(string);
}
