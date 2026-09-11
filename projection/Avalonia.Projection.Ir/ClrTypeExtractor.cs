using System;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;

namespace Avalonia.Projection.Ir;

public static class ClrTypeExtractor
{
    // NullabilityInfoContext caches per-module nullable-attribute state on an
    // instance-level dictionary that is not safe to mutate concurrently: two
    // threads calling Create() on the SAME shared instance at once can throw
    // "An item with the same key has already been added" from its internal
    // cache (this is why a single process-wide static instance previously
    // failed intermittently under parallel test execution). A [ThreadStatic]
    // field still reuses one instance across every call made from a single
    // thread (preserving the caching benefit for a sequential extraction
    // pass) while guaranteeing no two threads ever touch the same instance.
    [ThreadStatic]
    private static NullabilityInfoContext? _nullability;

    private static NullabilityInfoContext Nullability => _nullability ??= new NullabilityInfoContext();

    public static ProjectionIr Extract(IEnumerable<Type> sourceTypes, ProjectionPolicy policy)
    {
        var selected = sourceTypes
            .Where(policy.Includes)
            .Distinct()
            .OrderBy(t => t.FullName, StringComparer.Ordinal)
            .ToArray();
        var projectedNames = selected.ToDictionary(
            t => t,
            t => $"{policy.ProjectionNamespace}.IAvn{t.Name}");
        var types = new List<ProjectedType>(selected.Length);
        var skipped = new List<SkippedMember>();

        foreach (var type in selected)
            types.Add(ProjectType(type, selected, projectedNames, policy, skipped));
        var attachedProperties = ExtractAttachedProperties(selected, projectedNames, policy, skipped);
        var brushInterfaceName = BrushMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        var usesBrush = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.Brush));
        var commandInterfaceName = CommandMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        var commandHandlerInterfaceName = CommandMarshalling.QualifiedHandlerInterfaceName(policy.ProjectionNamespace);
        var usesCommand = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.Command));
        var templateInterfaceName = TemplateMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        var usesTemplate = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.DataTemplate));
        var itemFilterInterfaceName = FilterMarshalling.QualifiedItemFilterInterfaceName(policy.ProjectionNamespace);
        var textFilterInterfaceName = FilterMarshalling.QualifiedTextFilterInterfaceName(policy.ProjectionNamespace);
        var usesItemFilter = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.ItemFilter) ||
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.ItemFilter)));
        var usesTextFilter = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.TextFilter) ||
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.TextFilter)));
        var notificationInterfaceName = NotificationMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        var usesNotification = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.Notification) ||
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.Notification)));
        var itemSelectorInterfaceName = SelectorMarshalling.QualifiedItemSelectorInterfaceName(policy.ProjectionNamespace);
        var textSelectorInterfaceName = SelectorMarshalling.QualifiedTextSelectorInterfaceName(policy.ProjectionNamespace);
        var usesItemSelector = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.ItemSelector) ||
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.ItemSelector)));
        var usesTextSelector = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.TextSelector) ||
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.TextSelector)));
        var popupPlacementInterfaceName = PopupPlacementMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        var usesPopupPlacement = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.PopupPlacement));
        var asyncPopulatorInterfaceName = AsyncMarshalling.QualifiedAsyncPopulatorInterfaceName(policy.ProjectionNamespace);
        var usesAsyncPopulator = types.Any(type =>
            type.Properties.Any(property => property.Kind == MarshallingKind.AsyncPopulator));
        var dialogCompletionInterfaceName = AsyncMarshalling.QualifiedDialogCompletionInterfaceName(policy.ProjectionNamespace);
        var usesDialogCompletion = types.Any(type =>
            type.Methods.Any(method => method.Parameters.Any(p => p.Kind == MarshallingKind.DialogCompletion)));

        return new ProjectionIr
        {
            SourceAssembly = string.Join(
                ",",
                selected.Select(t => t.Assembly.GetName().Name).Distinct(StringComparer.Ordinal)),
            FactoryIid = CreateDeterministicIid(
                $"{policy.ProjectionNamespace}.IAvnControlFactory",
                policy.GetAbiVersion($"{policy.ProjectionNamespace}.IAvnControlFactory")),
            FactoryAbiVersion = policy.GetAbiVersion($"{policy.ProjectionNamespace}.IAvnControlFactory"),
            BrushInterfaceName = usesBrush ? brushInterfaceName : null,
            BrushInterfaceIid = usesBrush
                ? CreateDeterministicIid(brushInterfaceName, policy.GetAbiVersion(brushInterfaceName))
                : null,
            BrushAbiVersion = policy.GetAbiVersion(brushInterfaceName),
            CommandInterfaceName = usesCommand ? commandInterfaceName : null,
            CommandInterfaceIid = usesCommand
                ? CreateDeterministicIid(commandInterfaceName, policy.GetAbiVersion(commandInterfaceName))
                : null,
            CommandAbiVersion = policy.GetAbiVersion(commandInterfaceName),
            CommandHandlerInterfaceName = usesCommand ? commandHandlerInterfaceName : null,
            CommandHandlerInterfaceIid = usesCommand
                ? CreateDeterministicIid(commandHandlerInterfaceName, 1)
                : null,
            TemplateInterfaceName = usesTemplate ? templateInterfaceName : null,
            TemplateInterfaceIid = usesTemplate
                ? CreateDeterministicIid(templateInterfaceName, policy.GetAbiVersion(templateInterfaceName))
                : null,
            ItemFilterInterfaceName = usesItemFilter ? itemFilterInterfaceName : null,
            ItemFilterInterfaceIid = usesItemFilter
                ? CreateDeterministicIid(itemFilterInterfaceName, 1)
                : null,
            TextFilterInterfaceName = usesTextFilter ? textFilterInterfaceName : null,
            TextFilterInterfaceIid = usesTextFilter
                ? CreateDeterministicIid(textFilterInterfaceName, 1)
                : null,
            NotificationInterfaceName = usesNotification ? notificationInterfaceName : null,
            NotificationInterfaceIid = usesNotification
                ? CreateDeterministicIid(notificationInterfaceName, 1)
                : null,
            NotificationHandlerInterfaceIid = usesNotification
                ? CreateDeterministicIid($"{policy.ProjectionNamespace}.IAvnNotificationActionHandler", 1)
                : null,
            ItemSelectorInterfaceName = usesItemSelector ? itemSelectorInterfaceName : null,
            ItemSelectorInterfaceIid = usesItemSelector
                ? CreateDeterministicIid(itemSelectorInterfaceName, 1)
                : null,
            TextSelectorInterfaceName = usesTextSelector ? textSelectorInterfaceName : null,
            TextSelectorInterfaceIid = usesTextSelector
                ? CreateDeterministicIid(textSelectorInterfaceName, 1)
                : null,
            PopupPlacementInterfaceName = usesPopupPlacement ? popupPlacementInterfaceName : null,
            PopupPlacementInterfaceIid = usesPopupPlacement
                ? CreateDeterministicIid(popupPlacementInterfaceName, 1)
                : null,
            AsyncPopulatorInterfaceName = usesAsyncPopulator ? asyncPopulatorInterfaceName : null,
            AsyncPopulatorInterfaceIid = usesAsyncPopulator
                ? CreateDeterministicIid(asyncPopulatorInterfaceName, 1)
                : null,
            AsyncPopulatorCompletionInterfaceName = usesAsyncPopulator
                ? $"{policy.ProjectionNamespace}.{AsyncMarshalling.AsyncPopulatorCompletionInterfaceName}"
                : null,
            AsyncPopulatorCompletionInterfaceIid = usesAsyncPopulator
                ? CreateDeterministicIid($"{policy.ProjectionNamespace}.{AsyncMarshalling.AsyncPopulatorCompletionInterfaceName}")
                : null,
            DialogCompletionInterfaceName = usesDialogCompletion ? dialogCompletionInterfaceName : null,
            DialogCompletionInterfaceIid = usesDialogCompletion
                ? CreateDeterministicIid(dialogCompletionInterfaceName, 1)
                : null,
            Types = types,
            Enums = ExtractEnums(selected, policy),
            AttachedProperties = attachedProperties,
            Skipped = skipped,
        };
    }

    public static string CreateDeterministicIid(string projectedFullName, int abiVersion = 1)
    {
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(
            $"Avalonia.Rust.ABI/{projectedFullName}/v{abiVersion}"));
        Span<byte> guidBytes = stackalloc byte[16];
        hash.AsSpan(0, guidBytes.Length).CopyTo(guidBytes);
        guidBytes[7] = (byte)((guidBytes[7] & 0x0f) | 0x50);
        guidBytes[8] = (byte)((guidBytes[8] & 0x3f) | 0x80);
        return new Guid(guidBytes).ToString("D").ToUpperInvariant();
    }

    private static ProjectedType ProjectType(
        Type type,
        IReadOnlyCollection<Type> selected,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        List<SkippedMember> skipped)
    {
        var properties = new List<ProjectedProperty>();
        var methods = new List<ProjectedMethod>();
        var events = new List<ProjectedEvent>();
        var flags = BindingFlags.Public | BindingFlags.Instance;

        foreach (var property in type.GetProperties(flags).OrderBy(p => p.MetadataToken))
        {
            if (!policy.Includes(type, property))
            {
                if (property.DeclaringType == type)
                    Skip(skipped, type, property.Name,
                        policy.TryGetByDesignReason(property.Name, out var byDesign)
                            ? $"By design: {byDesign}"
                            : "Not included by projection policy");
                continue;
            }

            if (!TryMapMemberType(
                    property,
                    type,
                    property.PropertyType,
                    projectedNames,
                    policy,
                    out var kind,
                    out var interfaceName,
                    out var isNullable,
                    out var reason))
            {
                Skip(skipped, type, property.Name, reason!);
                continue;
            }

            properties.Add(new ProjectedProperty
            {
                Name = property.Name,
                Kind = kind,
                InterfaceName = interfaceName,
                InterfaceIid = kind is MarshallingKind.ComCollection or MarshallingKind.Brush
                    ? CreateDeterministicIid(interfaceName!, policy.GetAbiVersion(interfaceName!))
                    : null,
                InterfaceAbiVersion = kind is MarshallingKind.ComCollection or MarshallingKind.Brush
                    ? policy.GetAbiVersion(interfaceName!)
                    : 1,
                ElementInterfaceName = policy.TryGetOverride(type, property, out var memberOverride)
                    ? memberOverride.ElementInterfaceName
                    : null,
                ElementKind = policy.TryGetOverride(type, property, out memberOverride)
                    ? memberOverride.ElementKind
                    : null,
                StringConverterTypeName = policy.TryGetOverride(type, property, out memberOverride)
                    ? memberOverride.StringConverterTypeName
                    : null,
                HostImplementationTypeName = policy.TryGetOverride(type, property, out memberOverride)
                    ? memberOverride.HostImplementationTypeName
                    : null,
                ManagedTypeName = property.PropertyType.FullName,
                IsNullable = isNullable,
                CanRead = property.GetMethod?.IsPublic == true,
                CanWrite = property.SetMethod?.IsPublic == true,
            });
        }

        foreach (var @event in type.GetEvents(flags).OrderBy(e => e.MetadataToken))
        {
            if (!policy.Includes(type, @event))
            {
                if (@event.DeclaringType == type)
                    Skip(skipped, type, @event.Name,
                        policy.TryGetByDesignReason(@event.Name, out var byDesign)
                            ? $"By design: {byDesign}"
                            : "Not included by projection policy");
                continue;
            }

            if (!policy.TryGetEventOverride(type, @event, out var eventProjection))
            {
                Skip(skipped, type, @event.Name, "Included event requires an explicit event projection");
                continue;
            }

            if (@event.EventHandlerType?.GetMethod("Invoke")?.ReturnType != typeof(void))
            {
                Skip(skipped, type, @event.Name, "Event handler must return void");
                continue;
            }

            var handlerInterfaceName =
                $"{policy.ProjectionNamespace}.IAvn{type.Name}{@event.Name}Handler";
            var eventParameters = new List<ProjectedParameter>();
            if (eventProjection.PayloadKind == EventPayloadKind.Args)
            {
                var invokeParameters = @event.EventHandlerType!.GetMethod("Invoke")!.GetParameters();
                if (invokeParameters.Length < 2)
                {
                    Skip(skipped, type, @event.Name,
                        "Args event projection requires event arguments");
                    continue;
                }
                var eventArgsType = invokeParameters[1].ParameterType;
                var missing = eventProjection.Parameters
                    .Where(parameter => eventArgsType.GetProperty(
                        parameter.Name,
                        BindingFlags.Public | BindingFlags.Instance) is null)
                    .ToList();
                if (missing.Count > 0)
                {
                    Skip(skipped, type, @event.Name,
                        $"Args event argument property '{missing[0].Name}' was not found");
                    continue;
                }
                foreach (var parameter in eventProjection.Parameters)
                {
                    var property = eventArgsType.GetProperty(
                        parameter.Name,
                        BindingFlags.Public | BindingFlags.Instance)!;
                    eventParameters.Add(new ProjectedParameter
                    {
                        Name = property.Name,
                        Kind = MarshallingKind.Variant,
                        ManagedTypeName = property.PropertyType.FullName,
                        IsNullable = IsNullable(property),
                        Direction = ParameterDirection.In,
                    });
                }
            }
            if (eventProjection.PayloadKind == EventPayloadKind.Fields)
            {
                var invokeParameters = @event.EventHandlerType!.GetMethod("Invoke")!.GetParameters();
                if (invokeParameters.Length < 2)
                {
                    Skip(skipped, type, @event.Name, "Field event projection requires event arguments");
                    continue;
                }

                var eventArgsType = invokeParameters[1].ParameterType;
                foreach (var parameterProjection in eventProjection.Parameters)
                {
                    var property = eventArgsType.GetProperty(
                        parameterProjection.Name,
                        BindingFlags.Public | BindingFlags.Instance);
                    if (property is null)
                    {
                        Skip(
                            skipped,
                            type,
                            @event.Name,
                            $"Event argument property '{parameterProjection.Name}' was not found");
                        eventParameters.Clear();
                        break;
                    }

                    if (parameterProjection.Override is { } parameterOverride)
                    {
                        eventParameters.Add(new ProjectedParameter
                        {
                            Name = property.Name,
                            Kind = parameterOverride.Kind,
                            InterfaceName = parameterOverride.InterfaceName,
                            ManagedTypeName = property.PropertyType.FullName,
                            IsNullable = parameterOverride.IsNullable ?? IsNullable(property),
                            Direction = parameterProjection.Direction,
                            StringConverterTypeName = parameterOverride.StringConverterTypeName,
                        });
                        continue;
                    }

                    if (!TryMapType(
                            property.PropertyType,
                            projectedNames,
                            policy,
                            out var parameterKind,
                            out var parameterInterface,
                            out var parameterReason))
                    {
                        Skip(
                            skipped,
                            type,
                            @event.Name,
                            $"Event argument property '{parameterProjection.Name}': {parameterReason}");
                        eventParameters.Clear();
                        break;
                    }

                    if (parameterProjection.Direction == ParameterDirection.InOut &&
                        property.SetMethod?.IsPublic != true)
                    {
                        Skip(
                            skipped,
                            type,
                            @event.Name,
                            $"Event argument property '{parameterProjection.Name}' is not publicly writable");
                        eventParameters.Clear();
                        break;
                    }

                    eventParameters.Add(new ProjectedParameter
                    {
                        Name = property.Name,
                        Kind = parameterKind,
                        InterfaceName = parameterInterface,
                        ManagedTypeName = property.PropertyType.FullName,
                        IsNullable = IsNullable(property),
                        Direction = parameterProjection.Direction,
                    });
                }

                if (eventParameters.Count != eventProjection.Parameters.Count)
                    continue;
            }

            events.Add(new ProjectedEvent
            {
                Name = @event.Name,
                HandlerInterfaceName = handlerInterfaceName,
                HandlerInterfaceIid = CreateDeterministicIid(
                    handlerInterfaceName,
                    policy.GetAbiVersion(handlerInterfaceName)),
                HandlerInterfaceAbiVersion = policy.GetAbiVersion(handlerInterfaceName),
                PayloadKind = eventProjection.PayloadKind,
                ManagedHandlerTypeName = @event.EventHandlerType is { } handlerType
                    ? ManagedTypeName(handlerType)
                    : null,
                ArgsInterfaceName = eventProjection.PayloadKind == EventPayloadKind.Args
                    ? $"{policy.ProjectionNamespace}.IAvn{type.Name}{@event.Name}Args"
                    : null,
                ArgsInterfaceIid = eventProjection.PayloadKind == EventPayloadKind.Args
                    ? CreateDeterministicIid($"{policy.ProjectionNamespace}.IAvn{type.Name}{@event.Name}Args")
                    : null,
                Parameters = eventParameters,
            });
        }

        foreach (var method in type.GetMethods(flags)
                     .Where(m => !m.IsSpecialName)
                     .OrderBy(m => m.MetadataToken))
        {
            if (!policy.Includes(type, method))
            {
                if (method.DeclaringType == type)
                    Skip(skipped, type, FormatMethod(method),
                        policy.TryGetByDesignReason(method.Name, out var byDesign)
                            ? $"By design: {byDesign}"
                            : "Not included by projection policy");
                continue;
            }

            // A method that overrides a projected base method re-publishes the
            // same ABI slot: the base interface already carries it, so the
            // derived interface must not duplicate it. Only genuinely new
            // overloads (a different ABI name through their parameters)
            // project here.
            var abiName = AbiMethodName(method);
            var overridesProjectedBase =
                method.GetBaseDefinition() != method &&
                LineageProjectsMethod(type.BaseType, abiName, projectedNames, policy);
            if (overridesProjectedBase)
            {
                Skip(skipped, type, FormatMethod(method),
                    "By design: overrides the projected base slot");
                continue;
            }

            if (!TryProjectMethod(method, projectedNames, policy, out var projected, out var reason))
            {
                Skip(skipped, type, FormatMethod(method), reason!);
                continue;
            }

            methods.Add(projected);
        }

        var projectedFullName = projectedNames[type];
        return new ProjectedType
        {
            Name = $"IAvn{type.Name}",
            FullName = projectedFullName,
            ManagedFullName = type.FullName ?? type.Name,
            Kind = ProjectedTypeKind.Class,
            Iid = CreateDeterministicIid(
                projectedFullName,
                policy.GetProjectedTypeAbiVersion(projectedFullName)),
            AbiVersion = policy.GetProjectedTypeAbiVersion(projectedFullName),
            BaseFullName = FindProjectedBase(type.BaseType, selected, projectedNames),
            IsConstructible = !type.IsAbstract && type.GetConstructor(Type.EmptyTypes) is not null,
            Properties = properties,
            Methods = methods,
            Events = events,
        };
    }

    private static bool TryProjectMethod(
        MethodInfo method,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        out ProjectedMethod projected,
        out string? reason)
    {
        projected = null!;
        reason = null;

        var parameters = new List<ProjectedParameter>();
        foreach (var parameter in method.GetParameters())
        {
            var parameterType = parameter.ParameterType;
            if (parameterType.IsByRef)
            {
                reason = $"Parameter '{parameter.Name}' is by-ref";
                return false;
            }

            if (!TryMapType(
                    parameterType,
                    projectedNames,
                    policy,
                    out var kind,
                    out var interfaceName,
                    out var typeReason))
            {
                reason = parameter.Name is { } parameterName &&
                        policy.TryGetByDesignReason(parameterName, out var byDesign)
                    ? $"By design: {byDesign}"
                    : $"Parameter '{parameter.Name}': {typeReason}";
                return false;
            }

            parameters.Add(new ProjectedParameter
            {
                Name = parameter.Name ?? "arg",
                Kind = kind,
                InterfaceName = interfaceName,
                ManagedTypeName = parameterType.FullName,
                IsNullable = IsNullable(parameter),
                Direction = ParameterDirection.In,
            });
        }

        if (method.ReturnType != typeof(void))
        {
            if (!TryMapType(
                    method.ReturnType,
                    projectedNames,
                    policy,
                    out var returnKind,
                    out var returnInterface,
                    out var returnReason))
            {
                reason = $"Return type: {returnReason}";
                return false;
            }

            if (returnKind is not (
                MarshallingKind.I32 or MarshallingKind.I64 or MarshallingKind.F32 or
                MarshallingKind.F64 or MarshallingKind.Bool or MarshallingKind.StringUtf16 or
                MarshallingKind.ComInterface or MarshallingKind.ComCollection or
                MarshallingKind.Brush or MarshallingKind.Command or MarshallingKind.TimeSpanI64 or
                MarshallingKind.DateTimeI64 or MarshallingKind.PixelPointI32 or
                MarshallingKind.Variant))
            {
                reason = $"Return type '{method.ReturnType.FullName}' is not supported for projected commands";
                return false;
            }

            parameters.Add(new ProjectedParameter
            {
                Name = "value",
                Kind = returnKind,
                InterfaceName = returnInterface,
                ManagedTypeName = method.ReturnType.FullName,
                IsNullable = !method.ReturnType.IsValueType ||
                    Nullable.GetUnderlyingType(method.ReturnType) is not null,
                Direction = ParameterDirection.Out,
            });
        }

        projected = new ProjectedMethod
        {
            Name = AbiMethodName(method),
            ManagedName = method.Name,
            ReturnKind = MarshallingKind.I32,
            PreserveSig = true,
            Parameters = parameters,
        };
        return true;
    }

    private static bool TryMapMemberType(
        PropertyInfo member,
        Type projectionOwner,
        Type type,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        out MarshallingKind kind,
        out string? interfaceName,
        out bool isNullable,
        out string? reason)
    {
        if (policy.TryGetOverride(projectionOwner, member, out var value))
        {
            kind = value.Kind;
            interfaceName = value.InterfaceName;
            isNullable = value.IsNullable ?? IsNullable(member);
            reason = null;
            if (kind == MarshallingKind.ComInterface && string.IsNullOrWhiteSpace(interfaceName))
            {
                reason = "COM interface override requires InterfaceName";
                return false;
            }
            if (kind == MarshallingKind.ComCollection &&
                (string.IsNullOrWhiteSpace(interfaceName) || value.ElementKind is null ||
                 value.ElementKind == MarshallingKind.ComInterface &&
                 string.IsNullOrWhiteSpace(value.ElementInterfaceName)))
            {
                reason = "COM collection override requires InterfaceName, ElementKind, and an interface for COM elements";
                return false;
            }
            if (kind == MarshallingKind.ComCollection &&
                string.IsNullOrWhiteSpace(value.HostImplementationTypeName) &&
                value.ElementKind is not (MarshallingKind.StringUtf16 or MarshallingKind.ComInterface))
            {
                reason = "COM collection override with a non-host-implemented element kind supports string and COM elements";
                return false;
            }
            if (kind == MarshallingKind.StringUtf16 &&
                type != typeof(string) &&
                value.StringConverterTypeName is null &&
                !HasStringRoundTrip(type))
            {
                reason = "String override on a non-string member requires either a public static " +
                    "Parse(string) returning the member type with an overridden ToString(), or a " +
                    "StringConverterTypeName naming a host-side converter";
                return false;
            }
            return true;
        }

        isNullable = IsNullable(member);
        return TryMapType(type, projectedNames, policy, out kind, out interfaceName, out reason);
    }

    private static bool TryMapType(
        Type type,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        out MarshallingKind kind,
        out string? interfaceName,
        out string? reason)
    {
        interfaceName = null;
        reason = null;
        if (Nullable.GetUnderlyingType(type) == typeof(bool))
        {
            kind = MarshallingKind.NullableBool;
            return true;
        }
        type = Nullable.GetUnderlyingType(type) ?? type;

        if (type == typeof(int) || type.IsEnum)
            kind = MarshallingKind.I32;
        else if (type == typeof(long))
            kind = MarshallingKind.I64;
        else if (type == typeof(float))
            kind = MarshallingKind.F32;
        else if (type == typeof(double))
            kind = MarshallingKind.F64;
        else if (type == typeof(bool))
            kind = MarshallingKind.Bool;
        else if (type == typeof(char))
            kind = MarshallingKind.CharUtf16;
        else if (type == typeof(string))
            kind = MarshallingKind.StringUtf16;
        else if (type == typeof(object))
            kind = MarshallingKind.Variant;
        else if (type == typeof(TimeSpan))
            kind = MarshallingKind.TimeSpanI64;
        else if (type == typeof(DateTime))
            kind = MarshallingKind.DateTimeI64;
        else if (type == typeof(DateTimeOffset))
            kind = MarshallingKind.DateTimeI64;
        else if (type.FullName == "Avalonia.PixelPoint")
            kind = MarshallingKind.PixelPointI32;
        else if (GeometryMarshalling.TryGetByManagedTypeName(type.FullName, out var geometry))
            kind = geometry.Kind;
        else if (BrushMarshalling.IsBrush(type.FullName))
        {
            kind = MarshallingKind.Brush;
            interfaceName = BrushMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        }
        else if (CommandMarshalling.IsCommand(type.FullName))
        {
            kind = MarshallingKind.Command;
            interfaceName = CommandMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        }
        else if (TemplateMarshalling.IsDataTemplate(type.FullName))
        {
            kind = MarshallingKind.DataTemplate;
            interfaceName = TemplateMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        }
        else if (FilterMarshalling.IsItemFilter(type))
        {
            kind = MarshallingKind.ItemFilter;
            interfaceName = FilterMarshalling.QualifiedItemFilterInterfaceName(policy.ProjectionNamespace);
        }
        else if (FilterMarshalling.IsTextFilter(type))
        {
            kind = MarshallingKind.TextFilter;
            interfaceName = FilterMarshalling.QualifiedTextFilterInterfaceName(policy.ProjectionNamespace);
        }
        else if (SelectorMarshalling.IsItemSelector(type))
        {
            kind = MarshallingKind.ItemSelector;
            interfaceName = SelectorMarshalling.QualifiedItemSelectorInterfaceName(policy.ProjectionNamespace);
        }
        else if (SelectorMarshalling.IsTextSelector(type))
        {
            kind = MarshallingKind.TextSelector;
            interfaceName = SelectorMarshalling.QualifiedTextSelectorInterfaceName(policy.ProjectionNamespace);
        }
        else if (PopupPlacementMarshalling.IsPopupPlacementCallback(type))
        {
            kind = MarshallingKind.PopupPlacement;
            interfaceName = PopupPlacementMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        }
        else if (AsyncMarshalling.IsAsyncPopulator(type))
        {
            kind = MarshallingKind.AsyncPopulator;
            interfaceName = AsyncMarshalling.QualifiedAsyncPopulatorInterfaceName(policy.ProjectionNamespace);
        }
        else if (NotificationMarshalling.IsNotification(type.FullName))
        {
            kind = MarshallingKind.Notification;
            interfaceName = NotificationMarshalling.QualifiedInterfaceName(policy.ProjectionNamespace);
        }
        else if (projectedNames.TryGetValue(type, out interfaceName))
            kind = MarshallingKind.ComInterface;
        else
        {
            kind = MarshallingKind.Unsupported;
            reason = $"Type '{type.FullName}' is not marshallable";
            return false;
        }

        return true;
    }

    private static string? FindProjectedBase(
        Type? baseType,
        IReadOnlyCollection<Type> selected,
        IReadOnlyDictionary<Type, string> projectedNames)
    {
        while (baseType is not null)
        {
            if (selected.Contains(baseType))
                return projectedNames[baseType];
            baseType = baseType.BaseType;
        }
        return null;
    }

    /// <summary>
    /// Whether a base type in the projected lineage already publishes a method
    /// under the given ABI name, so an override must not re-publish it.
    /// </summary>
    private static bool LineageProjectsMethod(
        Type? baseType,
        string abiName,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy)
    {
        while (baseType is not null)
        {
            if (projectedNames.ContainsKey(baseType))
            {
                var flags = BindingFlags.Public | BindingFlags.Instance;
                foreach (var candidate in baseType.GetMethods(flags))
                {
                    if (candidate.IsSpecialName ||
                        !policy.Includes(baseType, candidate) ||
                        AbiMethodName(candidate) != abiName)
                        continue;
                    return true;
                }
            }
            baseType = baseType.BaseType;
        }
        return false;
    }

    private static bool IsNullable(PropertyInfo property) =>
        Nullability.Create(property).ReadState == NullabilityState.Nullable;

    /// <summary>
    /// A member whose CLR type is not <see cref="string"/> may still project as one, but only when
    /// the type itself owns both halves of the conversion: a public static <c>Parse(string)</c>
    /// returning the type, and a <c>ToString()</c> it overrides rather than inheriting from
    /// <see cref="object"/>. Anything else would leave the emitter guessing at a conversion.
    /// </summary>
    private static bool HasStringRoundTrip(Type type) =>
        type.GetMethod(
            "Parse",
            BindingFlags.Public | BindingFlags.Static,
            binder: null,
            [typeof(string)],
            modifiers: null) is { } parse &&
        parse.ReturnType == type &&
        type.GetMethod(
            nameof(ToString),
            BindingFlags.Public | BindingFlags.Instance,
            binder: null,
            Type.EmptyTypes,
            modifiers: null) is { DeclaringType: { } declaring } &&
        declaring != typeof(object);

    private static bool IsNullable(ParameterInfo parameter) =>
        Nullability.Create(parameter).ReadState == NullabilityState.Nullable;

    private static void Skip(List<SkippedMember> skipped, Type owner, string member, string reason) =>
        skipped.Add(new SkippedMember
        {
            Owner = owner.FullName ?? owner.Name,
            Member = member,
            Reason = reason,
        });

    private static string FormatMethod(MethodInfo method) =>
        $"{method.Name}({string.Join(", ", method.GetParameters().Select(p => p.ParameterType.Name))})";

    private static string AbiMethodName(MethodInfo method)
    {
        var parameters = method.GetParameters();
        return parameters.Length == 0
            ? method.Name
            : method.Name + "With" + string.Join("And", parameters.Select(p =>
                (Nullable.GetUnderlyingType(p.ParameterType) ?? p.ParameterType).Name));
    }

    private static IReadOnlyList<ProjectedEnum> ExtractEnums(
        IEnumerable<Type> selected,
        ProjectionPolicy policy)
    {
        var selectedTypes = selected.ToArray();
        return selectedTypes
            .SelectMany(type => type.GetProperties(BindingFlags.Public | BindingFlags.Instance)
                .Where(property => policy.Includes(type, property))
                .Select(property => Nullable.GetUnderlyingType(property.PropertyType) ?? property.PropertyType))
            .Concat(selectedTypes.SelectMany(type =>
                policy.AttachedProperties.TryGetValue(type.FullName ?? type.Name, out var names)
                    ? names.Where(name => !policy.TryGetAttachedOverride(type, name, out _))
                        .Select(name => type.GetMethod(
                            $"Get{name}",
                            BindingFlags.Public | BindingFlags.Static)?.ReturnType)
                        .Where(attachedType => attachedType is not null)
                        .Cast<Type>()
                    : []))
            .Concat(selectedTypes.SelectMany(type => type.GetEvents(BindingFlags.Public | BindingFlags.Instance)
                .Where(@event => policy.Includes(type, @event) &&
                    policy.TryGetEventOverride(type, @event, out var projection) &&
                    projection.PayloadKind == EventPayloadKind.Fields)
                .SelectMany(@event =>
                {
                    var eventArgsType = @event.EventHandlerType!.GetMethod("Invoke")!
                        .GetParameters()[1].ParameterType;
                    policy.TryGetEventOverride(type, @event, out var projection);
                    return projection.Parameters
                        .Select(parameter => eventArgsType.GetProperty(
                            parameter.Name,
                            BindingFlags.Public | BindingFlags.Instance)?.PropertyType)
                        .Where(parameterType => parameterType is not null)
                        .Select(parameterType => Nullable.GetUnderlyingType(parameterType!) ?? parameterType!);
                })))
            .Where(type => type.IsEnum)
            .Distinct()
            .OrderBy(type => type.FullName, StringComparer.Ordinal)
            .Select(type => new ProjectedEnum
            {
                Name = type.Name,
                FullName = type.FullName ?? type.Name,
                IsFlags = type.IsDefined(typeof(FlagsAttribute), inherit: false),
                Values = Enum.GetNames(type)
                    .Zip(Enum.GetValues(type).Cast<object>())
                    .Select(pair => new ProjectedEnumValue
                    {
                        Name = pair.First,
                        Value = Convert.ToInt32(pair.Second),
                    })
                    .ToArray(),
            })
            .ToArray();
    }

    private static IReadOnlyList<ProjectedAttachedProperty> ExtractAttachedProperties(
        IReadOnlyCollection<Type> selected,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        List<SkippedMember> skipped)
    {
        var result = new List<ProjectedAttachedProperty>();
        foreach (var (ownerName, names) in policy.AttachedProperties.OrderBy(p => p.Key, StringComparer.Ordinal))
        {
            var owner = selected.SingleOrDefault(type => type.FullName == ownerName);
            if (owner is null)
                continue;

            var staticsInterfaceName = $"{policy.ProjectionNamespace}.IAvn{owner.Name}Statics";
            foreach (var name in names)
            {
                var getter = owner.GetMethod(
                    $"Get{name}",
                    BindingFlags.Public | BindingFlags.Static);
                var setter = owner.GetMethod(
                    $"Set{name}",
                    BindingFlags.Public | BindingFlags.Static);
                if (getter is null || setter is null ||
                    getter.GetParameters().Length != 1 || setter.GetParameters().Length != 2 ||
                    getter.ReturnType != setter.GetParameters()[1].ParameterType)
                {
                    Skip(skipped, owner, name, "Attached property requires matching public static Get/Set methods");
                    continue;
                }

                var targetType = getter.GetParameters()[0].ParameterType;
                if (!projectedNames.Keys.Any(type => type.IsAssignableFrom(targetType)) &&
                    !projectedNames.ContainsKey(targetType))
                {
                    Skip(skipped, owner, name, $"Attached property target '{targetType.FullName}' is not projected");
                    continue;
                }

                if (!TryMapAttachedType(
                        owner,
                        name,
                        getter.ReturnType,
                        projectedNames,
                        policy,
                        out var kind,
                        out var isNullable,
                        out var converterTypeName,
                        out var reason))
                {
                    Skip(skipped, owner, name, reason!);
                    continue;
                }

                result.Add(new ProjectedAttachedProperty
                {
                    OwnerName = owner.Name,
                    OwnerManagedFullName = owner.FullName!,
                    StaticsInterfaceName = staticsInterfaceName,
                    StaticsInterfaceIid = CreateDeterministicIid(
                        staticsInterfaceName,
                        policy.GetAbiVersion(staticsInterfaceName)),
                    StaticsInterfaceAbiVersion = policy.GetAbiVersion(staticsInterfaceName),
                    Name = name,
                    Kind = kind,
                    ManagedTypeName = getter.ReturnType.FullName!,
                    IsNullable = isNullable,
                    StringConverterTypeName = converterTypeName,
                });
            }
        }
        return result;
    }

    private static bool TryMapAttachedType(
        Type owner,
        string name,
        Type returnType,
        IReadOnlyDictionary<Type, string> projectedNames,
        ProjectionPolicy policy,
        out MarshallingKind kind,
        out bool isNullable,
        out string? stringConverterTypeName,
        out string? reason)
    {
        if (policy.TryGetAttachedOverride(owner, name, out var value))
        {
            kind = value.Kind;
            isNullable = value.IsNullable ?? false;
            stringConverterTypeName = value.StringConverterTypeName;
            reason = null;
            if (kind != MarshallingKind.StringUtf16)
            {
                reason = $"Attached property override for '{owner.FullName}.{name}' must marshal as a string";
                return false;
            }
            if (returnType != typeof(string) &&
                stringConverterTypeName is null &&
                !HasStringRoundTrip(returnType))
            {
                reason = "String override on a non-string attached property requires either a public " +
                    "static Parse(string) returning the property type with an overridden ToString(), " +
                    "or a StringConverterTypeName naming a host-side converter";
                return false;
            }
            return true;
        }

        isNullable = false;
        stringConverterTypeName = null;
        return TryMapType(returnType, projectedNames, policy, out kind, out _, out reason);
    }

    private static string ManagedTypeName(Type type)
    {
        if (!type.IsGenericType)
            return type.FullName ?? type.Name;

        var definitionName = type.GetGenericTypeDefinition().FullName!;
        definitionName = definitionName[..definitionName.IndexOf('`')];
        return $"{definitionName}<{string.Join(", ", type.GetGenericArguments().Select(ManagedTypeName))}>";
    }
}
