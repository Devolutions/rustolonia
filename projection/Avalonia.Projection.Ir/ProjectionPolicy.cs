using System.Reflection;

namespace Avalonia.Projection.Ir;

public sealed class ProjectionPolicy
{
    public IReadOnlyList<string> IncludeNamespaces { get; init; } = [];
    public IReadOnlyList<string> IncludeTypeNames { get; init; } = [];
    public IReadOnlyList<string> ExcludeTypeNames { get; init; } = [];
    public IReadOnlyDictionary<string, IReadOnlyList<string>> IncludeMembers { get; init; } =
        new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal);
    public IReadOnlyDictionary<string, MarshallingOverride> MemberOverrides { get; init; } =
        new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal);
    public IReadOnlyDictionary<string, EventProjection> EventOverrides { get; init; } =
        new Dictionary<string, EventProjection>(StringComparer.Ordinal);
    public IReadOnlyDictionary<string, IReadOnlyList<string>> AttachedProperties { get; init; } =
        new Dictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal);

    /// <summary>
    /// Marshalling overrides for attached properties, keyed by
    /// <c>{owner full name}.{attached property name}</c>. An attached property whose CLR type
    /// is not directly marshallable needs one of these or it is reported as a gap.
    /// </summary>
    public IReadOnlyDictionary<string, MarshallingOverride> AttachedPropertyOverrides { get; init; } =
        new Dictionary<string, MarshallingOverride>(StringComparer.Ordinal);

    public IReadOnlyDictionary<string, int> AbiVersions { get; init; } =
        new Dictionary<string, int>(StringComparer.Ordinal);
    public int DefaultProjectedTypeAbiVersion { get; init; } = 1;
    public string ProjectionNamespace { get; init; } = "Avalonia.Host.Com";

    public bool Includes(Type type)
    {
        var fullName = type.FullName ?? type.Name;
        if (ExcludeTypeNames.Contains(fullName, StringComparer.Ordinal))
            return false;
        if (IncludeTypeNames.Contains(fullName, StringComparer.Ordinal))
            return true;
        if (IncludeNamespaces.Count == 0 && IncludeTypeNames.Count == 0)
            return true;
        var ns = type.Namespace ?? "";
        return IncludeNamespaces.Any(n =>
            ns.Equals(n, StringComparison.Ordinal) ||
            ns.StartsWith(n + ".", StringComparison.Ordinal));
    }

    public bool Includes(Type projectionOwner, MemberInfo member)
    {
        var owner = projectionOwner.FullName ?? projectionOwner.Name;
        return IncludeMembers.TryGetValue(owner, out var members) &&
            members.Contains(member.Name, StringComparer.Ordinal);
    }

    /// <summary>
    /// Members the projection deliberately never publishes, with the reason
    /// they are excluded by design rather than blocked. Members named here
    /// appear in the gap report under "By design: reason" so a reader can
    /// tell an architectural exclusion from a marshalling limitation.
    /// </summary>
    public IReadOnlyDictionary<string, string> ByDesignMembers { get; init; } =
        new Dictionary<string, string>(StringComparer.Ordinal);

    public bool TryGetByDesignReason(string memberName, out string? reason)
    {
        if (ByDesignMembers.TryGetValue(memberName, out reason))
            return true;
        // The AvaloniaProperty plumbing and equality members are excluded
        // across every type without enumerating them per owner.
        if (memberName is "Bind" or "GetValue" or "SetValue" or "SetCurrentValue" or
            "ClearValue" or "GetBaseValue" or "IsSet" or "IsAnimating" or
            "CheckAccess" or "VerifyAccess" or "Equals" or "GetHashCode" or "ToString" or
            "BeginInit" or "EndInit" or "UpdateSelectionFromEvent" or "Item" or
            "Dispatcher" or "PropertyChanged" or "onClick" or "onClosing")
        {
            reason = memberName switch
            {
                "Bind" => "the property system is not projected; use the imperative surface",
                "GetValue" or "SetValue" or "SetCurrentValue" or "ClearValue" or
                    "GetBaseValue" or "IsSet" or "IsAnimating" =>
                    "AvaloniaProperty plumbing is not projected; use the named member instead",
                "CheckAccess" or "VerifyAccess" or "Dispatcher" =>
                    "threading is owned by the host",
                "Equals" or "GetHashCode" or "ToString" =>
                    "identity and formatting members are not ABI surface",
                "BeginInit" or "EndInit" or "UpdateSelectionFromEvent" =>
                    "initialization and internal plumbing is owned by the host",
                "Item" => "the indexer is not an ABI member",
                "onClick" or "onClosing" =>
                    "System.Action parameters ride the notification CCW's handler slots instead",
                _ => "not part of the projected model",
            };
            return true;
        }
        reason = null;
        return false;
    }

    public bool TryGetOverride(Type projectionOwner, MemberInfo member, out MarshallingOverride value) =>
        MemberOverrides.TryGetValue(
            $"{projectionOwner.FullName}.{member.Name}",
            out value!);

    public bool TryGetEventOverride(Type projectionOwner, EventInfo member, out EventProjection value) =>
        EventOverrides.TryGetValue(
            $"{projectionOwner.FullName}.{member.Name}",
            out value!);

    public bool TryGetAttachedOverride(Type owner, string attachedPropertyName, out MarshallingOverride value) =>
        AttachedPropertyOverrides.TryGetValue(
            $"{owner.FullName}.{attachedPropertyName}",
            out value!);

    public int GetAbiVersion(string projectedInterfaceName) =>
        AbiVersions.TryGetValue(projectedInterfaceName, out var version) ? version : 1;

    public int GetProjectedTypeAbiVersion(string projectedInterfaceName) =>
        AbiVersions.TryGetValue(projectedInterfaceName, out var version)
            ? version
            : DefaultProjectedTypeAbiVersion;
}

public sealed class EventProjection
{
    public required EventPayloadKind PayloadKind { get; init; }
    public IReadOnlyList<EventParameterProjection> Parameters { get; init; } = [];
}

public sealed class EventParameterProjection
{
    public required string Name { get; init; }
    public ParameterDirection Direction { get; init; } = ParameterDirection.In;
}

public sealed class MarshallingOverride
{
    public required MarshallingKind Kind { get; init; }
    public string? InterfaceName { get; init; }
    public string? ElementInterfaceName { get; init; }
    public MarshallingKind? ElementKind { get; init; }
    public bool? IsNullable { get; init; }

    /// <summary>
    /// A host-side static class that owns both halves of the string conversion for a member
    /// whose CLR type cannot own them itself — an interface, or <c>object</c>. It must expose
    /// <c>public static string? ToAbi(T value)</c> and <c>public static T FromAbi(string? value)</c>.
    /// Only meaningful when <see cref="Kind"/> is <see cref="MarshallingKind.StringUtf16"/>.
    /// </summary>
    public string? StringConverterTypeName { get; init; }

    /// <summary>
    /// The host-side class implementing the generated collection interface when the managed
    /// collection needs adaptation the generated wrapper cannot assume. It must expose
    /// <c>public static IAvnX? FromManaged(T? value)</c> and
    /// <c>public static T? ToManaged(IAvnX? value)</c>. Only meaningful when
    /// <see cref="Kind"/> is <see cref="MarshallingKind.ComCollection"/>.
    /// </summary>
    public string? HostImplementationTypeName { get; init; }
}
