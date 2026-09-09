namespace Avalonia.Projection.Ir;

public sealed partial class ProjectionIr
{
    private static readonly MarshallingKind[] CollectionElementKinds =
    [
        MarshallingKind.ComInterface,
        MarshallingKind.StringUtf16,
        MarshallingKind.Variant,
        MarshallingKind.DateTimeI64,
    ];

    public void Validate()
    {
        if (Version < MinimumVersion || Version > CurrentVersion)
        {
            throw new InvalidOperationException(
                $"Unsupported projection IR version {Version} (supported {MinimumVersion}..={CurrentVersion}).");
        }

        var types = RequireCollection(Types, "types");
        var typesByFullName = new Dictionary<string, ProjectedType>(StringComparer.Ordinal);
        for (var typeIndex = 0; typeIndex < types.Count; typeIndex++)
        {
            var type = RequireElement(types, typeIndex, "types");
            if (string.IsNullOrWhiteSpace(type.FullName))
                throw new InvalidOperationException($"Projected type at types[{typeIndex}] is missing fullName.");
            if (!typesByFullName.TryAdd(type.FullName, type))
                throw new InvalidOperationException($"Duplicate projected type '{type.FullName}'.");
            if (!Enum.IsDefined(type.Kind))
                throw new InvalidOperationException($"Unknown type kind '{type.Kind}' at type '{type.FullName}'.");

            var methods = RequireCollection(type.Methods, $"types[{typeIndex}].methods");
            for (var methodIndex = 0; methodIndex < methods.Count; methodIndex++)
            {
                var method = RequireElement(methods, methodIndex, $"types[{typeIndex}].methods");
                var methodLocation = $"type '{type.FullName}' method '{method.Name}'";
                ValidateKind(method.ReturnKind, methodLocation);
                var parameters = RequireCollection(method.Parameters, $"types[{typeIndex}].methods[{methodIndex}].parameters");
                for (var parameterIndex = 0; parameterIndex < parameters.Count; parameterIndex++)
                {
                    var parameter = RequireElement(parameters, parameterIndex, $"types[{typeIndex}].methods[{methodIndex}].parameters");
                    ValidateParameter(parameter, methodLocation);
                    if (parameter.Direction == ParameterDirection.InOut)
                        throw new InvalidOperationException($"InOut method parameters are not supported at {methodLocation} parameter '{parameter.Name}'.");
                }
                if (method.ReturnKind != MarshallingKind.I32 || !method.PreserveSig)
                    throw new InvalidOperationException($"Only PreserveSig I32 HRESULT returns are supported at {methodLocation}.");
            }

            var properties = RequireCollection(type.Properties, $"types[{typeIndex}].properties");
            for (var propertyIndex = 0; propertyIndex < properties.Count; propertyIndex++)
            {
                var property = RequireElement(properties, propertyIndex, $"types[{typeIndex}].properties");
                ValidateProperty(property, $"type '{type.FullName}' property '{property.Name}'");
            }

            var events = RequireCollection(type.Events, $"types[{typeIndex}].events");
            for (var eventIndex = 0; eventIndex < events.Count; eventIndex++)
            {
                var @event = RequireElement(events, eventIndex, $"types[{typeIndex}].events");
                ValidateEvent(@event, type, $"types[{typeIndex}].events[{eventIndex}].parameters");
            }
        }

        var attached = RequireCollection(AttachedProperties, "attachedProperties");
        for (var attachedIndex = 0; attachedIndex < attached.Count; attachedIndex++)
        {
            var property = RequireElement(attached, attachedIndex, "attachedProperties");
            ValidateKind(
                property.Kind,
                $"attached property '{property.OwnerName}.{property.Name}'");
        }

        RequireCollection(Enums, "enums");
        RequireCollection(Skipped, "skipped");

        foreach (var type in typesByFullName.Values)
            ValidateInheritance(type, typesByFullName);
    }

    private static void ValidateProperty(ProjectedProperty property, string location)
    {
        ValidateKind(property.Kind, location);
        if (property.Kind != MarshallingKind.ComCollection)
        {
            if (property.ElementKind is { } elementKind)
                ValidateKind(elementKind, location + " element");
            return;
        }

        if (string.IsNullOrWhiteSpace(property.InterfaceName))
            throw new InvalidOperationException($"Missing interfaceName at {location}.");
        if (string.IsNullOrWhiteSpace(property.InterfaceIid))
            throw new InvalidOperationException($"Missing interfaceIid at {location}.");
        if (property.ElementKind is not { } collectionElementKind)
            throw new InvalidOperationException($"Missing elementKind at {location}.");
        if (Array.IndexOf(CollectionElementKinds, collectionElementKind) < 0)
        {
            throw new InvalidOperationException(
                $"Unsupported collection element kind '{collectionElementKind}' at {location}.");
        }

        if (collectionElementKind == MarshallingKind.ComInterface
            && string.IsNullOrWhiteSpace(property.ElementInterfaceName))
        {
            throw new InvalidOperationException($"Missing elementInterfaceName at {location}.");
        }
    }

    private static void ValidateEvent(ProjectedEvent @event, ProjectedType type, string parametersPath)
    {
        var location = $"type '{type.FullName}' event '{@event.Name}'";
        if (!Enum.IsDefined(@event.PayloadKind))
        {
            throw new InvalidOperationException(
                $"Unknown event payload kind '{@event.PayloadKind}' at {location}.");
        }

        var parameters = RequireCollection(@event.Parameters, parametersPath);
        for (var parameterIndex = 0; parameterIndex < parameters.Count; parameterIndex++)
        {
            var parameter = RequireElement(parameters, parameterIndex, parametersPath);
            ValidateParameter(parameter, location);
        }

        switch (@event.PayloadKind)
        {
            case EventPayloadKind.None when parameters.Count > 0:
                throw new InvalidOperationException($"None payload cannot have parameters at {location}.");
            case EventPayloadKind.Fields when parameters.Count == 0:
                throw new InvalidOperationException($"Fields payload requires parameters at {location}.");
            case EventPayloadKind.Args when string.IsNullOrWhiteSpace(@event.ArgsInterfaceName):
                throw new InvalidOperationException($"Missing argsInterfaceName at {location}.");
            case EventPayloadKind.Args when string.IsNullOrWhiteSpace(@event.ArgsInterfaceIid):
                throw new InvalidOperationException($"Missing argsInterfaceIid at {location}.");
        }
    }

    private static void ValidateParameter(ProjectedParameter parameter, string ownerLocation)
    {
        var location = $"{ownerLocation} parameter '{parameter.Name}'";
        ValidateKind(parameter.Kind, location);
        if (!Enum.IsDefined(parameter.Direction))
        {
            throw new InvalidOperationException(
                $"Unknown parameter direction '{parameter.Direction}' at {location}.");
        }
    }

    private static void ValidateKind(MarshallingKind kind, string location)
    {
        if (!Enum.IsDefined(kind) || kind == MarshallingKind.Unsupported)
        {
            throw new InvalidOperationException(
                $"Unknown marshalling kind '{kind}' at {location}.");
        }
    }

    private static void ValidateInheritance(
        ProjectedType type,
        IReadOnlyDictionary<string, ProjectedType> typesByFullName)
    {
        if (type.BaseFullName is null)
            return;
        if (string.IsNullOrWhiteSpace(type.BaseFullName))
            throw new InvalidOperationException($"Type '{type.FullName}' has an empty baseFullName.");

        if (string.Equals(type.BaseFullName, type.FullName, StringComparison.Ordinal))
            throw new InvalidOperationException($"Type '{type.FullName}' has a self base reference.");

        var seen = new List<string> { type.FullName };
        var current = type;
        while (current.BaseFullName is not null)
        {
            if (string.IsNullOrWhiteSpace(current.BaseFullName))
                throw new InvalidOperationException($"Type '{current.FullName}' has an empty baseFullName.");
            if (!typesByFullName.TryGetValue(current.BaseFullName, out var baseType))
            {
                throw new InvalidOperationException(
                    $"Type '{type.FullName}' references missing base '{current.BaseFullName}'.");
            }

            if (seen.Contains(baseType.FullName, StringComparer.Ordinal))
            {
                seen.Add(baseType.FullName);
                throw new InvalidOperationException($"Inheritance cycle: {string.Join(" -> ", seen)}.");
            }

            seen.Add(baseType.FullName);
            current = baseType;
        }
    }

    private static IReadOnlyList<T> RequireCollection<T>(IReadOnlyList<T>? items, string path)
    {
        if (items is null)
            throw new InvalidOperationException($"Projection IR '{path}' must not be null.");
        return items;
    }

    private static T RequireElement<T>(IReadOnlyList<T> items, int index, string path) where T : class
    {
        return items[index]
            ?? throw new InvalidOperationException($"Projection IR '{path}[{index}]' must not be null.");
    }
}
