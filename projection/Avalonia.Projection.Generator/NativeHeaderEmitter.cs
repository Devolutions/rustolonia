using System.Globalization;
using System.Text;
using Avalonia.Projection.Ir;

namespace Avalonia.Projection.Generator;

public static class NativeHeaderEmitter
{
    public static string Emit(ProjectionIr ir)
    {
        var events = ir.Types
            .SelectMany(type => type.Events)
            .GroupBy(@event => @event.HandlerInterfaceName, StringComparer.Ordinal)
            .Select(group => group.First())
            .OrderBy(@event => @event.HandlerInterfaceName, StringComparer.Ordinal)
            .ToArray();
        var collections = ir.Types
            .SelectMany(type => type.Properties)
            .Where(property => property.Kind == MarshallingKind.ComCollection)
            .GroupBy(property => property.InterfaceName, StringComparer.Ordinal)
            .Select(group => group.First())
            .OrderBy(property => property.InterfaceName, StringComparer.Ordinal)
            .ToArray();
        var types = ir.Types
            .Where(type => type.Kind == ProjectedTypeKind.Class)
            .OrderBy(type => type.FullName, StringComparer.Ordinal)
            .ToArray();
        var statics = ir.AttachedProperties
            .GroupBy(property => property.StaticsInterfaceName, StringComparer.Ordinal)
            .OrderBy(group => group.Key, StringComparer.Ordinal)
            .ToArray();

        var names = events.Select(@event => SimpleName(@event.HandlerInterfaceName))
            .Concat(events
                .Where(@event => @event.ArgsInterfaceName is not null)
                .Select(@event => SimpleName(@event.ArgsInterfaceName!)))
            .Concat(collections.Select(property => SimpleName(property.InterfaceName!)))
            .Concat(types.Select(type => type.Name))
            .Concat(statics.Select(group => SimpleName(group.Key)))
            .Concat(ir.BrushInterfaceName is null ? [] : new[] { SimpleName(ir.BrushInterfaceName) })
            .Concat(ir.GradientBrushInterfaceName is null
                ? []
                : new[]
                {
                    SimpleName(ir.GradientBrushInterfaceName),
                    SimpleName(ir.LinearGradientBrushInterfaceName!),
                    SimpleName(ir.RadialGradientBrushInterfaceName!),
                })
            .Concat(ir.CommandInterfaceName is null
                ? []
                : new[]
                {
                    SimpleName(ir.CommandInterfaceName),
                    CommandMarshalling.HandlerInterfaceName,
                })
            .Append("IAvnControlFactory")
            .Distinct(StringComparer.Ordinal)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();

        var sb = new StringBuilder();
        sb.AppendLine("/* Generated from projection.ir.json. Do not edit. */");
        sb.AppendLine("#ifndef AVALONIA_RUST_ABI_H");
        sb.AppendLine("#define AVALONIA_RUST_ABI_H");
        sb.AppendLine();
        sb.AppendLine("#include <stdint.h>");
        sb.AppendLine();
        sb.AppendLine("#if defined(_WIN32)");
        sb.AppendLine("#define AVN_CALL __stdcall");
        sb.AppendLine("#else");
        sb.AppendLine("#define AVN_CALL");
        sb.AppendLine("#endif");
        sb.AppendLine();
        sb.AppendLine("typedef int32_t AvnHResult;");
        sb.AppendLine("typedef struct AvnGuid {");
        sb.AppendLine("    uint32_t data1;");
        sb.AppendLine("    uint16_t data2;");
        sb.AppendLine("    uint16_t data3;");
        sb.AppendLine("    uint8_t data4[8];");
        sb.AppendLine("} AvnGuid;");
        sb.AppendLine();

        foreach (var geometry in GeometryMarshalling.All)
        {
            sb.AppendLine($"/* Blittable ABI mirror of {geometry.ManagedTypeName}. */");
            sb.AppendLine($"typedef struct {geometry.AbiName} {{");
            foreach (var field in geometry.Fields)
                sb.AppendLine($"    {NativeFieldType(field.Kind)} {field.NativeName};");
            sb.AppendLine($"}} {geometry.AbiName};");
            sb.AppendLine();
            sb.AppendLine($"/* Nullable ABI wrapper of {geometry.AbiName}. has_value is 0 or 1. */");
            sb.AppendLine($"typedef struct {geometry.OptionalAbiName} {{");
            sb.AppendLine("    int32_t has_value;");
            sb.AppendLine($"    {geometry.AbiName} value;");
            sb.AppendLine($"}} {geometry.OptionalAbiName};");
            sb.AppendLine();
        }

        sb.AppendLine("/* Blittable ABI mirror of Avalonia.PixelPoint. */");
        sb.AppendLine("typedef struct AvnPixelPoint {");
        sb.AppendLine("    int32_t x;");
        sb.AppendLine("    int32_t y;");
        sb.AppendLine("} AvnPixelPoint;");
        sb.AppendLine();
        sb.AppendLine("/* Nullable ABI wrapper of a DateTime tick count. has_value is 0 or 1. */");
        sb.AppendLine("typedef struct AvnOptionalDateTime {");
        sb.AppendLine("    int32_t has_value;");
        sb.AppendLine("    int64_t ticks;");
        sb.AppendLine("} AvnOptionalDateTime;");
        sb.AppendLine();
        sb.AppendLine("/* Nullable ABI wrapper of a TimeSpan tick count. has_value is 0 or 1. */");
        sb.AppendLine("typedef struct AvnOptionalTimeSpan {");
        sb.AppendLine("    int32_t has_value;");
        sb.AppendLine("    int64_t ticks;");
        sb.AppendLine("} AvnOptionalTimeSpan;");
        sb.AppendLine();

        if (ir.GradientBrushInterfaceName is not null)
        {
            var colorAbiName = GeometryMarshalling.All
                .Single(geometry => geometry.Kind == MarshallingKind.Color).AbiName;
            sb.AppendLine("/* Blittable ABI mirror of Avalonia.RelativePoint. unit: 0 relative, 1 absolute. */");
            sb.AppendLine("typedef struct AvnRelativePoint {");
            sb.AppendLine("    double x;");
            sb.AppendLine("    double y;");
            sb.AppendLine("    int32_t unit;");
            sb.AppendLine("} AvnRelativePoint;");
            sb.AppendLine();
            sb.AppendLine("/* Blittable ABI mirror of Avalonia.RelativeScalar. unit: 0 relative, 1 absolute. */");
            sb.AppendLine("typedef struct AvnRelativeScalar {");
            sb.AppendLine("    double scalar;");
            sb.AppendLine("    int32_t unit;");
            sb.AppendLine("} AvnRelativeScalar;");
            sb.AppendLine();
            sb.AppendLine("/* Blittable ABI mirror of a single gradient stop (offset plus packed colour). */");
            sb.AppendLine("typedef struct AvnGradientStop {");
            sb.AppendLine("    double offset;");
            sb.AppendLine($"    {colorAbiName} color;");
            sb.AppendLine("} AvnGradientStop;");
            sb.AppendLine();
            sb.AppendLine($"/* Fixed-capacity buffer of up to {GradientBrushMarshalling.MaxStops} AvnGradientStop entries. */");
            sb.AppendLine("typedef struct AvnGradientStopBuffer {");
            sb.AppendLine($"    AvnGradientStop stops[{GradientBrushMarshalling.MaxStops}];");
            sb.AppendLine("} AvnGradientStopBuffer;");
            sb.AppendLine();
        }

        sb.AppendLine("/* Tagged scalar carrying object? command parameters. */");
        sb.AppendLine("/* tag: 0 none, 1 utf16, 2 i32, 3 f64, 4 bool. */");
        sb.AppendLine("typedef struct AvnVariant {");
        sb.AppendLine("    int32_t tag;");
        sb.AppendLine("    const uint16_t* utf16;");
        sb.AppendLine("    int32_t i32;");
        sb.AppendLine("    double f64;");
        sb.AppendLine("} AvnVariant;");
        sb.AppendLine();

        foreach (var name in names)
        {
            sb.AppendLine($"typedef struct {name} {name};");
            sb.AppendLine($"typedef struct {name}Vtbl {name}Vtbl;");
        }
        sb.AppendLine();

        foreach (var @event in events)
        {
            var name = SimpleName(@event.HandlerInterfaceName);
            if (@event.PayloadKind == EventPayloadKind.Args && @event.ArgsInterfaceName is { } argsFullName)
            {
                // The args interface: one count/getter pair per projected property.
                var argsName = SimpleName(argsFullName);
                EmitIid(sb, argsName, @event.ArgsInterfaceIid!, 1);
                BeginInterface(sb, argsName);
                var slot = 3;
                foreach (var parameter in @event.Parameters)
                {
                    EmitSlot(sb, slot, $"get_{Snake(parameter.Name)}_count", argsName, ["int32_t* value"]);
                    slot++;
                    EmitSlot(sb, slot, $"get_{Snake(parameter.Name)}_at", argsName, ["int32_t index", "AvnVariant* value"]);
                    slot++;
                }
                EndInterface(sb, argsName, slot);

                EmitIid(sb, name, @event.HandlerInterfaceIid, @event.HandlerInterfaceAbiVersion);
                BeginInterface(sb, name);
                EmitSlot(sb, 3, "invoke", name, [$"{argsName}* args"]);
                EndInterface(sb, name, 4);
                continue;
            }
            var arguments = @event.Parameters.Select(EventParameter).ToArray();
            EmitIid(sb, name, @event.HandlerInterfaceIid, @event.HandlerInterfaceAbiVersion);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name, arguments);
            EndInterface(sb, name, 4);
        }

        if (ir.BrushInterfaceName is { } brushInterfaceName)
        {
            var brushName = SimpleName(brushInterfaceName);
            var colorAbiName = GeometryMarshalling.All
                .Single(geometry => geometry.Kind == MarshallingKind.Color).AbiName;
            EmitIid(sb, brushName, ir.BrushInterfaceIid!, ir.BrushAbiVersion);
            BeginInterface(sb, brushName);
            EmitSlot(sb, 3, "get_color", brushName, [$"{colorAbiName}* value"]);
            EmitSlot(sb, 4, "get_opacity", brushName, ["double* value"]);
            EndInterface(sb, brushName, 5);
        }

        if (ir.GradientBrushInterfaceName is { } gradientBrushInterfaceName)
        {
            var gradientName = SimpleName(gradientBrushInterfaceName);
            var linearName = SimpleName(ir.LinearGradientBrushInterfaceName!);
            var radialName = SimpleName(ir.RadialGradientBrushInterfaceName!);

            EmitIid(sb, gradientName, ir.GradientBrushInterfaceIid!, 1);
            BeginInterface(sb, gradientName);
            EmitSlot(sb, 3, "get_opacity", gradientName, ["double* value"]);
            EmitSlot(sb, 4, "get_spread_method", gradientName, ["int32_t* value"]);
            EmitSlot(sb, 5, "get_stop_count", gradientName, ["int32_t* value"]);
            EmitSlot(sb, 6, "get_stops", gradientName, ["AvnGradientStopBuffer* value"]);
            EndInterface(sb, gradientName, 7);

            // Derived gradient interfaces flatten the base's four slots ahead of their own,
            // mirroring the single-inheritance vtable layout produced for control interfaces.
            EmitIid(sb, linearName, ir.LinearGradientBrushInterfaceIid!, 1);
            BeginInterface(sb, linearName);
            EmitSlot(sb, 3, "get_opacity", linearName, ["double* value"]);
            EmitSlot(sb, 4, "get_spread_method", linearName, ["int32_t* value"]);
            EmitSlot(sb, 5, "get_stop_count", linearName, ["int32_t* value"]);
            EmitSlot(sb, 6, "get_stops", linearName, ["AvnGradientStopBuffer* value"]);
            EmitSlot(sb, 7, "get_start_point", linearName, ["AvnRelativePoint* value"]);
            EmitSlot(sb, 8, "get_end_point", linearName, ["AvnRelativePoint* value"]);
            EndInterface(sb, linearName, 9);

            EmitIid(sb, radialName, ir.RadialGradientBrushInterfaceIid!, 1);
            BeginInterface(sb, radialName);
            EmitSlot(sb, 3, "get_opacity", radialName, ["double* value"]);
            EmitSlot(sb, 4, "get_spread_method", radialName, ["int32_t* value"]);
            EmitSlot(sb, 5, "get_stop_count", radialName, ["int32_t* value"]);
            EmitSlot(sb, 6, "get_stops", radialName, ["AvnGradientStopBuffer* value"]);
            EmitSlot(sb, 7, "get_center", radialName, ["AvnRelativePoint* value"]);
            EmitSlot(sb, 8, "get_gradient_origin", radialName, ["AvnRelativePoint* value"]);
            EmitSlot(sb, 9, "get_radius_x", radialName, ["AvnRelativeScalar* value"]);
            EmitSlot(sb, 10, "get_radius_y", radialName, ["AvnRelativeScalar* value"]);
            EndInterface(sb, radialName, 11);
        }

        if (ir.CommandInterfaceName is { } commandInterfaceName)
        {
            var commandName = SimpleName(commandInterfaceName);
            var handlerName = CommandMarshalling.HandlerInterfaceName;
            var ns = commandInterfaceName[..commandInterfaceName.LastIndexOf('.')];
            var handlerIid = ClrTypeExtractor.CreateDeterministicIid(
                CommandMarshalling.QualifiedHandlerInterfaceName(ns));
            EmitIid(sb, handlerName, handlerIid, 1);
            BeginInterface(sb, handlerName);
            EmitSlot(sb, 3, "invoke", handlerName, []);
            EndInterface(sb, handlerName, 4);
            EmitIid(sb, commandName, ir.CommandInterfaceIid!, ir.CommandAbiVersion);
            BeginInterface(sb, commandName);
            EmitSlot(sb, 3, "execute", commandName, ["AvnVariant parameter"]);
            EmitSlot(sb, 4, "can_execute", commandName, ["AvnVariant parameter", "int32_t* value"]);
            EmitSlot(sb, 5, "advise_can_execute_changed", commandName,
                [$"{handlerName}* handler", "int64_t* subscription_id"]);
            EmitSlot(sb, 6, "unadvise_can_execute_changed", commandName, ["int64_t subscription_id"]);
            EndInterface(sb, commandName, 7);
        }

        if (ir.TemplateInterfaceName is { } templateInterfaceName)
        {
            var templateName = SimpleName(templateInterfaceName);
            EmitIid(sb, templateName, ir.TemplateInterfaceIid!, 1);
            BeginInterface(sb, templateName);
            EmitSlot(sb, 3, "match", templateName, ["AvnVariant data", "int32_t* value"]);
            EmitSlot(sb, 4, "build", templateName, ["IAvnControl** value"]);
            EndInterface(sb, templateName, 5);
        }

        if (ir.ItemFilterInterfaceName is { } itemFilterInterfaceName)
        {
            var name = SimpleName(itemFilterInterfaceName);
            EmitIid(sb, name, ir.ItemFilterInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name, ["const uint16_t* search", "AvnVariant item", "int32_t* result"]);
            EndInterface(sb, name, 4);
        }

        if (ir.TextFilterInterfaceName is { } textFilterInterfaceName)
        {
            var name = SimpleName(textFilterInterfaceName);
            EmitIid(sb, name, ir.TextFilterInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name, ["const uint16_t* search", "const uint16_t* item", "int32_t* result"]);
            EndInterface(sb, name, 4);
        }

        if (ir.ItemSelectorInterfaceName is { } itemSelectorInterfaceName)
        {
            var name = SimpleName(itemSelectorInterfaceName);
            EmitIid(sb, name, ir.ItemSelectorInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name, ["const uint16_t* search", "AvnVariant item", "uint16_t** text"]);
            EndInterface(sb, name, 4);
        }

        if (ir.TextSelectorInterfaceName is { } textSelectorInterfaceName)
        {
            var name = SimpleName(textSelectorInterfaceName);
            EmitIid(sb, name, ir.TextSelectorInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name, ["const uint16_t* search", "const uint16_t* item", "uint16_t** text"]);
            EndInterface(sb, name, 4);
        }

        if (ir.PopupPlacementInterfaceName is { } popupPlacementInterfaceName)
        {
            var name = SimpleName(popupPlacementInterfaceName);
            EmitIid(sb, name, ir.PopupPlacementInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "invoke", name,
                [
                    "double popup_width",
                    "double popup_height",
                    "double anchor_x",
                    "double anchor_y",
                    "double anchor_width",
                    "double anchor_height",
                    "double* offset_x",
                    "double* offset_y",
                    "int32_t* anchor",
                    "int32_t* gravity",
                    "int32_t* constraint_adjustment",
                ]);
            EndInterface(sb, name, 4);
        }

        if (ir.AsyncPopulatorInterfaceName is { } asyncPopulatorInterfaceName)
        {
            var completionName = "IAvnAsyncPopulatorCompletion";
            EmitIid(sb, completionName, ir.AsyncPopulatorCompletionInterfaceIid!, 1);
            BeginInterface(sb, completionName);
            EmitSlot(sb, 3, "complete", completionName,
                ["int64_t request_id", "int32_t hresult", "IAvnVariantList* items"]);
            EndInterface(sb, completionName, 4);

            var name = SimpleName(asyncPopulatorInterfaceName);
            EmitIid(sb, name, ir.AsyncPopulatorInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "begin_populate", name,
                ["int64_t request_id", "IAvnAsyncPopulatorCompletion* completion", "const uint16_t* search_text"]);
            EndInterface(sb, name, 4);
        }

        if (ir.NotificationInterfaceName is { } notificationInterfaceName)
        {
            var name = SimpleName(notificationInterfaceName);
            var ns = notificationInterfaceName[..notificationInterfaceName.LastIndexOf('.')];
            var handlerName = "IAvnNotificationActionHandler";
            var handlerIid = ClrTypeExtractor.CreateDeterministicIid($"{ns}.{handlerName}");
            EmitIid(sb, handlerName, handlerIid, 1);
            BeginInterface(sb, handlerName);
            EmitSlot(sb, 3, "invoke", handlerName, []);
            EndInterface(sb, handlerName, 4);
            EmitIid(sb, name, ir.NotificationInterfaceIid!, 1);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "get_title", name, ["const uint16_t** value"]);
            EmitSlot(sb, 4, "get_message", name, ["const uint16_t** value"]);
            EmitSlot(sb, 5, "get_type", name, ["int32_t* value"]);
            EmitSlot(sb, 6, "get_expiration", name, ["int64_t* value"]);
            EmitSlot(sb, 7, "get_on_click", name, [$"{handlerName}** value"]);
            EmitSlot(sb, 8, "get_on_close", name, [$"{handlerName}** value"]);
            EndInterface(sb, name, 9);
        }

        foreach (var collection in collections)
        {
            var name = SimpleName(collection.InterfaceName!);
            var element = AbiType(
                collection.ElementKind!.Value,
                collection.ElementInterfaceName,
                pointerForInterface: true);
            EmitIid(sb, name, collection.InterfaceIid!, collection.InterfaceAbiVersion);
            BeginInterface(sb, name);
            EmitSlot(sb, 3, "get_count", name, ["int32_t* value"]);
            EmitSlot(sb, 4, "get_at", name, ["int32_t index", $"{element}* value"]);
            EmitSlot(sb, 5, "add", name, [$"{InputType(collection.ElementKind.Value, collection.ElementInterfaceName)} value"]);
            EmitSlot(sb, 6, "index_of", name,
                [$"{InputType(collection.ElementKind.Value, collection.ElementInterfaceName)} value", "int32_t* index"]);
            EmitSlot(sb, 7, "remove_at", name, ["int32_t index"]);
            EmitSlot(sb, 8, "clear", name, []);
            EndInterface(sb, name, 9);
        }

        foreach (var type in types)
        {
            EmitIid(sb, type.Name, type.Iid!, type.AbiVersion);
            BeginInterface(sb, type.Name);
            var slot = 3;
            EmitSlot(sb, slot++, "get_object_id", type.Name, ["int64_t* value"]);
            EmitSlot(sb, slot++, "get_lifetime_token", type.Name, ["int64_t* value"]);
            foreach (var owner in Lineage(ir, type))
            {
                foreach (var property in owner.Properties)
                {
                    if (property.CanRead)
                    {
                        EmitSlot(
                            sb,
                            slot++,
                            $"get_{Snake(property.Name)}",
                            type.Name,
                            [$"{OutputType(property.Kind, property.InterfaceName, property.IsNullable)} value"]);
                    }
                    if (property.CanWrite)
                    {
                        EmitSlot(
                            sb,
                            slot++,
                            $"set_{Snake(property.Name)}",
                            type.Name,
                            [$"{InputType(property.Kind, property.InterfaceName, property.IsNullable)} value"]);
                    }
                }
                foreach (var method in owner.Methods)
                {
                    EmitSlot(
                        sb,
                        slot++,
                        Snake(method.Name),
                        type.Name,
                        method.Parameters.Select(MethodParameter).ToArray());
                }
                foreach (var @event in owner.Events)
                {
                    EmitSlot(
                        sb,
                        slot++,
                        $"advise_{Snake(@event.Name)}",
                        type.Name,
                        [$"{SimpleName(@event.HandlerInterfaceName)}* handler", "int64_t* subscription_id"]);
                    EmitSlot(
                        sb,
                        slot++,
                        $"unadvise_{Snake(@event.Name)}",
                        type.Name,
                        ["int64_t subscription_id"]);
                }
            }
            EndInterface(sb, type.Name, slot);
        }

        foreach (var group in statics)
        {
            var name = SimpleName(group.Key);
            EmitIid(
                sb,
                name,
                group.First().StaticsInterfaceIid,
                group.First().StaticsInterfaceAbiVersion);
            BeginInterface(sb, name);
            var slot = 3;
            foreach (var property in group)
            {
                EmitSlot(
                    sb,
                    slot++,
                    $"get_{Snake(property.Name)}",
                    name,
                    ["IAvnControl* target", $"{OutputType(property.Kind, null, property.IsNullable)} value"]);
                EmitSlot(
                    sb,
                    slot++,
                    $"set_{Snake(property.Name)}",
                    name,
                    ["IAvnControl* target", $"{InputType(property.Kind, null, property.IsNullable)} value"]);
            }
            EndInterface(sb, name, slot);
        }

        EmitIid(sb, "IAvnControlFactory", ir.FactoryIid!, ir.FactoryAbiVersion);
        BeginInterface(sb, "IAvnControlFactory");
        var factorySlot = 3;
        foreach (var type in types.Where(type => type.IsConstructible))
        {
            EmitSlot(
                sb,
                factorySlot++,
                $"create_{Snake(type.Name[4..])}",
                "IAvnControlFactory",
                [$"{type.Name}** value"]);
        }
        foreach (var group in statics)
        {
            var name = SimpleName(group.Key);
            EmitSlot(
                sb,
                factorySlot++,
                $"get_{Snake(group.First().OwnerName)}_statics",
                "IAvnControlFactory",
                [$"{name}** value"]);
        }
        if (ir.BrushInterfaceName is { } factoryBrushInterfaceName)
        {
            var brushName = SimpleName(factoryBrushInterfaceName);
            var colorAbiName = GeometryMarshalling.All
                .Single(geometry => geometry.Kind == MarshallingKind.Color).AbiName;
            EmitSlot(
                sb,
                factorySlot++,
                Snake(BrushMarshalling.FactoryMethodName),
                "IAvnControlFactory",
                [$"{colorAbiName} color", "double opacity", $"{brushName}** value"]);
        }
        if (ir.GradientBrushInterfaceName is not null)
        {
            var linearName = SimpleName(ir.LinearGradientBrushInterfaceName!);
            var radialName = SimpleName(ir.RadialGradientBrushInterfaceName!);
            EmitSlot(
                sb,
                factorySlot++,
                Snake(GradientBrushMarshalling.CreateLinearFactoryMethodName),
                "IAvnControlFactory",
                [
                    "double opacity",
                    "int32_t spread_method",
                    "int32_t stop_count",
                    "AvnGradientStopBuffer stops",
                    "AvnRelativePoint start_point",
                    "AvnRelativePoint end_point",
                    $"{linearName}** value",
                ]);
            EmitSlot(
                sb,
                factorySlot++,
                Snake(GradientBrushMarshalling.CreateRadialFactoryMethodName),
                "IAvnControlFactory",
                [
                    "double opacity",
                    "int32_t spread_method",
                    "int32_t stop_count",
                    "AvnGradientStopBuffer stops",
                    "AvnRelativePoint center",
                    "AvnRelativePoint gradient_origin",
                    "AvnRelativeScalar radius_x",
                    "AvnRelativeScalar radius_y",
                    $"{radialName}** value",
                ]);
        }
        EndInterface(sb, "IAvnControlFactory", factorySlot);

        sb.AppendLine("#endif /* AVALONIA_RUST_ABI_H */");
        return sb.ToString();
    }

    private static void BeginInterface(StringBuilder sb, string name)
    {
        sb.AppendLine($"struct {name}Vtbl {{");
        sb.AppendLine($"    AvnHResult (AVN_CALL *query_interface)({name}* self, const AvnGuid* iid, void** result); /* slot 0 */");
        sb.AppendLine($"    uint32_t (AVN_CALL *add_ref)({name}* self); /* slot 1 */");
        sb.AppendLine($"    uint32_t (AVN_CALL *release)({name}* self); /* slot 2 */");
    }

    private static void EndInterface(StringBuilder sb, string name, int slots)
    {
        sb.AppendLine("};");
        sb.AppendLine($"struct {name} {{ const {name}Vtbl* vtbl; }};");
        sb.AppendLine($"#define {Snake(name).ToUpperInvariant()}_VTABLE_SLOTS {slots}");
        sb.AppendLine();
    }

    private static void EmitSlot(
        StringBuilder sb,
        int slot,
        string method,
        string owner,
        IReadOnlyList<string> parameters)
    {
        var suffix = parameters.Count == 0 ? "" : ", " + string.Join(", ", parameters);
        sb.AppendLine(
            $"    AvnHResult (AVN_CALL *{method})({owner}* self{suffix}); /* slot {slot} */");
    }

    private static void EmitIid(StringBuilder sb, string name, string value, int abiVersion)
    {
        var guid = Guid.Parse(value);
        var bytes = guid.ToByteArray();
        sb.AppendLine($"static const AvnGuid {Snake(name).ToUpperInvariant()}_IID = {{");
        sb.AppendLine($"    0x{guid.ToString("N", CultureInfo.InvariantCulture)[..8].ToUpperInvariant()},");
        sb.AppendLine($"    0x{guid.ToString("N", CultureInfo.InvariantCulture)[8..12].ToUpperInvariant()},");
        sb.AppendLine($"    0x{guid.ToString("N", CultureInfo.InvariantCulture)[12..16].ToUpperInvariant()},");
        sb.Append("    { ");
        sb.Append(string.Join(", ", bytes[8..].Select(value => $"0x{value:X2}")));
        sb.AppendLine(" }");
        sb.AppendLine("};");
        sb.AppendLine($"#define {Snake(name).ToUpperInvariant()}_ABI_VERSION {abiVersion}");
    }

    private static IReadOnlyList<ProjectedType> Lineage(ProjectionIr ir, ProjectedType type)
    {
        var result = new List<ProjectedType> { type };
        var current = type;
        while (current.BaseFullName is { } baseName)
        {
            current = ir.Types.Single(candidate => candidate.FullName == baseName);
            result.Add(current);
        }
        result.Reverse();
        return result;
    }

    private static string EventParameter(ProjectedParameter parameter) =>
        parameter.Direction == ParameterDirection.InOut
            ? $"{AbiType(parameter.Kind, parameter.InterfaceName, pointerForInterface: false, parameter.IsNullable)}* {Snake(parameter.Name)}"
            : $"{InputType(parameter.Kind, parameter.InterfaceName, parameter.IsNullable)} {Snake(parameter.Name)}";

    private static string MethodParameter(ProjectedParameter parameter) =>
        parameter.Direction switch
        {
            ParameterDirection.Out =>
                $"{OutputType(parameter.Kind, parameter.InterfaceName, parameter.IsNullable)} {Snake(parameter.Name)}",
            ParameterDirection.InOut =>
                $"{AbiType(parameter.Kind, parameter.InterfaceName, pointerForInterface: false, parameter.IsNullable)}* {Snake(parameter.Name)}",
            _ => $"{InputType(parameter.Kind, parameter.InterfaceName, parameter.IsNullable)} {Snake(parameter.Name)}",
        };

    private static string OutputType(MarshallingKind kind, string? interfaceName, bool isNullable = false) =>
        kind switch
        {
            MarshallingKind.StringUtf16 => "uint16_t**",
            MarshallingKind.ComInterface or MarshallingKind.ComCollection =>
                $"{SimpleName(interfaceName!)}**",
            MarshallingKind.Brush => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.Command => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.DataTemplate => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.ItemFilter => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.TextFilter => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.ItemSelector => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.TextSelector => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.PopupPlacement => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.AsyncPopulator => $"{SimpleName(interfaceName!)}**",
            MarshallingKind.Notification => $"{SimpleName(interfaceName!)}**",
            _ => $"{AbiType(kind, interfaceName, pointerForInterface: false, isNullable)}*",
        };

    private static string InputType(MarshallingKind kind, string? interfaceName, bool isNullable = false) =>
        kind switch
        {
            MarshallingKind.StringUtf16 => "const uint16_t*",
            MarshallingKind.ComInterface or MarshallingKind.ComCollection =>
                $"{SimpleName(interfaceName!)}*",
            MarshallingKind.Brush => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.Command => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.DataTemplate => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.ItemFilter => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.TextFilter => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.ItemSelector => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.TextSelector => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.PopupPlacement => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.AsyncPopulator => $"{SimpleName(interfaceName!)}*",
            MarshallingKind.Notification => $"{SimpleName(interfaceName!)}*",
            _ => AbiType(kind, interfaceName, pointerForInterface: false, isNullable),
        };

    private static string AbiType(
        MarshallingKind kind,
        string? interfaceName,
        bool pointerForInterface,
        bool isNullable = false) =>
        kind switch
        {
            MarshallingKind.CharUtf16 => "uint16_t",
            MarshallingKind.I32 or MarshallingKind.Bool or MarshallingKind.NullableBool => "int32_t",
            MarshallingKind.I64 => "int64_t",
            MarshallingKind.TimeSpanI64 => isNullable ? "AvnOptionalTimeSpan" : "int64_t",
            MarshallingKind.DateTimeI64 => isNullable ? "AvnOptionalDateTime" : "int64_t",
            MarshallingKind.PixelPointI32 => "AvnPixelPoint",
            MarshallingKind.F32 => "float",
            MarshallingKind.F64 => "double",
            MarshallingKind.StringUtf16 => "uint16_t*",
            MarshallingKind.ComInterface or MarshallingKind.ComCollection =>
                SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.Brush => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.Command => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.DataTemplate => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.ItemFilter => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.TextFilter => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.ItemSelector => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.TextSelector => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.PopupPlacement => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.AsyncPopulator => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.Notification => SimpleName(interfaceName!) + (pointerForInterface ? "*" : ""),
            MarshallingKind.Variant => "AvnVariant",
            _ when GeometryMarshalling.TryGet(kind, out var geometry) =>
                isNullable ? geometry.OptionalAbiName : geometry.AbiName,
            _ => throw new InvalidOperationException($"Unsupported ABI kind '{kind}'."),
        };

    private static string NativeFieldType(GeometryFieldKind kind) =>
        kind switch
        {
            GeometryFieldKind.Double => "double",
            GeometryFieldKind.UInt32 => "uint32_t",
            _ => throw new InvalidOperationException($"Unsupported ABI field kind '{kind}'."),
        };

    private static string SimpleName(string fullName) =>
        fullName[(fullName.LastIndexOf('.') + 1)..];

    private static string Snake(string value)
    {
        var sb = new StringBuilder();
        for (var index = 0; index < value.Length; index++)
        {
            var character = value[index];
            if (char.IsUpper(character) && index > 0 &&
                (!char.IsUpper(value[index - 1]) ||
                 index + 1 < value.Length && char.IsLower(value[index + 1])))
            {
                sb.Append('_');
            }
            sb.Append(char.ToLowerInvariant(character));
        }
        return sb.ToString();
    }
}
