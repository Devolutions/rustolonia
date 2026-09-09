namespace Avalonia.Projection.Ir;

/// <summary>
/// The object-model projection of the async-completion surfaces: AutoCompleteBox's
/// async populator delegate and Window's modal dialog completion.
/// </summary>
public static class AsyncMarshalling
{
    public const string AsyncPopulatorManagedTypeName =
        "System.Func`3[[System.String, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e],[System.Threading.CancellationToken, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e],[System.Threading.Tasks.Task`1[[System.Collections.Generic.IEnumerable`1[[System.Object, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e]], System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e]], System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e]]]";

    public const string AsyncPopulatorInterfaceName = "IAvnAsyncPopulator";
    public const string AsyncPopulatorCompletionInterfaceName = "IAvnAsyncPopulatorCompletion";
    public const string DialogCompletionInterfaceName = "IAvnDialogCompletion";

    public static string QualifiedAsyncPopulatorInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{AsyncPopulatorInterfaceName}";

    public static string QualifiedDialogCompletionInterfaceName(string projectionNamespace) =>
        $"{projectionNamespace}.{DialogCompletionInterfaceName}";

    public static bool IsAsyncPopulator(Type? type) =>
        type is not null &&
        type.IsGenericType &&
        type.FullName == AsyncPopulatorManagedTypeName;
}
