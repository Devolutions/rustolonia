namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of AutoCompleteBox's selector delegates, which
/// format an item into the text shown in the drop-down for a search.
/// </summary>
public static class SelectorMarshalling
{
    public const string ItemSelectorManagedTypeName =
        "Avalonia.Controls.AutoCompleteSelector`1";
    public const string TextSelectorManagedTypeName =
        "Avalonia.Controls.AutoCompleteSelector`1";

    public const string ItemSelectorInterfaceName = "IAvnItemSelector";
    public const string TextSelectorInterfaceName = "IAvnTextSelector";

    public static string QualifiedItemSelectorInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{ItemSelectorInterfaceName}";

    public static string QualifiedTextSelectorInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{TextSelectorInterfaceName}";

    public static bool IsItemSelector(Type? type) =>
        type is not null &&
        type.IsGenericType &&
        type.GetGenericTypeDefinition().FullName == ItemSelectorManagedTypeName &&
        type.GetGenericArguments()[0] == typeof(object);

    public static bool IsTextSelector(Type? type) =>
        type is not null &&
        type.IsGenericType &&
        type.GetGenericTypeDefinition().FullName == TextSelectorManagedTypeName &&
        type.GetGenericArguments()[0] == typeof(string);
}
