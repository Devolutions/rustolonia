using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Projection.Ir;

public static class ComInterfaceExtractor
{
    public static ProjectionIr Extract(Assembly assembly, ProjectionPolicy? policy = null)
    {
        policy ??= new ProjectionPolicy();
        var types = new List<ProjectedType>();
        var skipped = new List<SkippedMember>();

        foreach (var type in assembly.GetExportedTypes().OrderBy(t => t.FullName, StringComparer.Ordinal))
        {
            if (!policy.Includes(type))
                continue;

            if (!IsGeneratedComInterface(type))
            {
                if (type.IsInterface)
                {
                    skipped.Add(new SkippedMember
                    {
                        Owner = type.FullName ?? type.Name,
                        Member = type.Name,
                        Reason = "Not a GeneratedComInterface",
                    });
                }
                continue;
            }

            types.Add(ProjectInterface(type));
        }

        return new ProjectionIr
        {
            SourceAssembly = assembly.GetName().Name,
            Types = types,
            Skipped = skipped,
        };
    }

    private static bool IsGeneratedComInterface(Type type) =>
        type.IsInterface && type.GetCustomAttributesData().Any(a =>
            a.AttributeType.Name is nameof(GeneratedComInterfaceAttribute) or "GeneratedComInterfaceAttribute");

    private static ProjectedType ProjectInterface(Type type)
    {
        var iid = type.GetCustomAttribute<GuidAttribute>()?.Value;
        var methods = new List<ProjectedMethod>();

        foreach (var method in type.GetMethods().OrderBy(m => m.MetadataToken))
        {
            if (!TryProjectMethod(method, out var projected, out var reason))
            {
                throw new InvalidOperationException(
                    $"Cannot project COM interface '{type.FullName}' method '{method.Name}': {reason}. " +
                    "Omitting a method would change its published vtable.");
            }

            methods.Add(projected);
        }

        var baseInterface = type.GetInterfaces().FirstOrDefault(IsGeneratedComInterface);

        return new ProjectedType
        {
            Name = type.Name,
            FullName = type.FullName ?? type.Name,
            Kind = ProjectedTypeKind.Interface,
            Iid = iid,
            BaseFullName = baseInterface?.FullName,
            Methods = methods,
        };
    }

    private static bool TryProjectMethod(
        MethodInfo method,
        out ProjectedMethod projected,
        out string? reason)
    {
        projected = null!;
        reason = null;

        if (method.IsSpecialName || method.IsStatic || method.IsGenericMethod ||
            method.ReturnType != typeof(int) ||
            method.GetCustomAttribute<PreserveSigAttribute>() is null)
        {
            reason = "Only ordinary instance methods returning PreserveSig I32 HRESULT are supported";
            return false;
        }

        if (!TryMapType(method.ReturnType, out var returnKind, out _, out var returnReason))
        {
            reason = $"Return type: {returnReason}";
            return false;
        }

        var parameters = new List<ProjectedParameter>();
        foreach (var p in method.GetParameters())
        {
            var pType = p.ParameterType;
            var dir = ParameterDirection.In;
            if (pType.IsByRef)
            {
                pType = pType.GetElementType()!;
                dir = p.IsOut && !p.IsIn ? ParameterDirection.Out : ParameterDirection.InOut;
            }

            if (dir == ParameterDirection.InOut || Nullable.GetUnderlyingType(pType) is not null)
            {
                reason = $"Parameter '{p.Name}': InOut and nullable value parameters are not supported";
                return false;
            }

            if (!TryMapType(pType, out var kind, out var iface, out var paramReason))
            {
                reason = $"Parameter '{p.Name}': {paramReason}";
                return false;
            }

            parameters.Add(new ProjectedParameter
            {
                Name = p.Name ?? "arg",
                Kind = kind,
                Direction = dir,
                InterfaceName = iface,
                IsNullable = IsNullable(p, pType),
            });
        }

        projected = new ProjectedMethod
        {
            Name = method.Name,
            ReturnKind = returnKind,
            PreserveSig = method.GetCustomAttribute<PreserveSigAttribute>() is not null,
            Parameters = parameters,
        };
        return true;
    }

    private static bool TryMapType(Type type, out MarshallingKind kind, out string? interfaceName, out string? reason)
    {
        interfaceName = null;
        reason = null;
        type = Nullable.GetUnderlyingType(type) ?? type;

        if (type == typeof(void))
        {
            kind = MarshallingKind.Void;
            return true;
        }
        if (type == typeof(int))
        {
            kind = MarshallingKind.I32;
            return true;
        }
        if (type == typeof(long))
        {
            kind = MarshallingKind.I64;
            return true;
        }
        if (type == typeof(float))
        {
            kind = MarshallingKind.F32;
            return true;
        }
        if (type == typeof(double))
        {
            kind = MarshallingKind.F64;
            return true;
        }
        if (type == typeof(bool))
        {
            kind = MarshallingKind.Bool;
            return true;
        }
        if (type == typeof(string))
        {
            kind = MarshallingKind.StringUtf16;
            return true;
        }
        if (type.IsInterface && IsGeneratedComInterface(type))
        {
            kind = MarshallingKind.ComInterface;
            interfaceName = type.FullName;
            return true;
        }
        if (GeometryMarshalling.TryGetByAbiName(type.Name, out var geometry))
        {
            kind = geometry.Kind;
            return true;
        }

        kind = MarshallingKind.Unsupported;
        reason = $"Type '{type.FullName}' is not marshallable";
        return false;
    }

    private static bool IsNullable(ParameterInfo p, Type type)
    {
        if (!type.IsValueType)
            return true;
        return Nullable.GetUnderlyingType(p.ParameterType.IsByRef ? p.ParameterType.GetElementType()! : p.ParameterType) is not null;
    }
}
