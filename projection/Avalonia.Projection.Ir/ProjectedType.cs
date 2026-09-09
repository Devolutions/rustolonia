namespace Avalonia.Projection.Ir;

public sealed class ProjectedType
{
    public required string Name { get; init; }
    public required string FullName { get; init; }
    public required ProjectedTypeKind Kind { get; init; }
    public string? Iid { get; init; }
    public int AbiVersion { get; init; } = 1;
    public string? BaseFullName { get; init; }
    public string? ManagedFullName { get; init; }
    public bool IsConstructible { get; init; }
    public IReadOnlyList<ProjectedMethod> Methods { get; init; } = [];
    public IReadOnlyList<ProjectedProperty> Properties { get; init; } = [];
    public IReadOnlyList<ProjectedEvent> Events { get; init; } = [];
}

public sealed class ProjectedProperty
{
    public required string Name { get; init; }
    public required MarshallingKind Kind { get; init; }
    public bool CanRead { get; init; }
    public bool CanWrite { get; init; }
    public string? InterfaceName { get; init; }
    public string? InterfaceIid { get; init; }
    public int InterfaceAbiVersion { get; init; } = 1;
    public string? ElementInterfaceName { get; init; }
    public MarshallingKind? ElementKind { get; init; }
    public string? ManagedTypeName { get; init; }
    public bool IsNullable { get; init; }

    /// <summary>
    /// The host-side static class that converts this member between its CLR type and the
    /// string that crosses the ABI. Set only when <see cref="Kind"/> is
    /// <see cref="MarshallingKind.StringUtf16"/> and the CLR type owns no string round trip
    /// of its own. Purely a host-side detail: the ABI slot is an ordinary UTF-16 string, so
    /// neither the native header nor the Rust bindings look at it.
    /// </summary>
    public string? StringConverterTypeName { get; init; }

    /// <summary>
    /// The host-side class implementing this collection interface, set only when the
    /// managed collection needs adaptation semantics the generated wrapper cannot assume
    /// (an enumerable source with copy-on-write mutation, say). The interface is still
    /// generated; the class lives in the host assembly.
    /// </summary>
    public string? HostImplementationTypeName { get; init; }
}
