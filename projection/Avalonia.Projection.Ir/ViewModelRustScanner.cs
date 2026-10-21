using System.Globalization;
using System.IO;
using System.Text.RegularExpressions;

namespace Avalonia.Projection.Ir;

/// <summary>
/// Scans Rust source files for <c>#[view_model]</c>, <c>#[view_model_enum]</c>,
/// <c>#[prop]</c>, <c>#[collection]</c>, and <c>#[command]</c> annotations and
/// produces or updates a canonical <see cref="ViewModelIr"/> with deterministic IDs.
/// </summary>
public static class ViewModelRustScanner
{
    public static ViewModelIr ScanFileOrDirectory(string path, ViewModelIr? baseline = null)
    {
        if (File.Exists(path))
            return ScanSources([File.ReadAllText(path)], baseline);

        if (Directory.Exists(path))
        {
            var files = Directory
                .EnumerateFiles(path, "*.rs", SearchOption.AllDirectories)
                .OrderBy(file => file, StringComparer.Ordinal)
                .Select(File.ReadAllText)
                .ToArray();
            return ScanSources(files, baseline);
        }

        throw new FileNotFoundException($"Rust source path '{path}' does not exist.", path);
    }

    public static ViewModelIr ScanSources(IEnumerable<string> sources, ViewModelIr? baseline = null)
    {
        var combined = string.Join("\n", sources);
        var enums = new List<ViewModelEnumDefinition>();
        if (baseline is not null)
            enums.AddRange(baseline.Enums);

        foreach (Match enumMatch in EnumBlockRegex.Matches(combined))
        {
            var attrText = enumMatch.Groups["attr"].Value;
            var enumName = enumMatch.Groups["name"].Value;
            var body = enumMatch.Groups["body"].Value;
            var attrs = ParseAttributes(attrText);
            var existing = enums.FirstOrDefault(e => e.Name == enumName);
            var ns = GetAttr(attrs, "namespace")
                ?? existing?.ManagedNamespace
                ?? "Avalonia.Rust.Generated";
            var id = GetIntAttr(attrs, "id")
                ?? existing?.Id
                ?? NextId(enums.Select(e => e.Id));

            var members = new List<ViewModelEnumMember>();
            long nextValue = 0;
            foreach (Match memberMatch in EnumMemberRegex.Matches(body))
            {
                var memberName = memberMatch.Groups["name"].Value;
                if (string.IsNullOrWhiteSpace(memberName))
                    continue;
                var valueGroup = memberMatch.Groups["value"].Value;
                var value = !string.IsNullOrWhiteSpace(valueGroup)
                    ? long.Parse(valueGroup, CultureInfo.InvariantCulture)
                    : nextValue;
                nextValue = value + 1;
                members.Add(new ViewModelEnumMember { Name = memberName, Value = value });
            }

            var definition = new ViewModelEnumDefinition
            {
                Id = id,
                Name = enumName,
                ManagedNamespace = ns,
                Members = members,
            };

            var existingIndex = enums.FindIndex(e => e.Name == enumName);
            if (existingIndex >= 0)
                enums[existingIndex] = definition;
            else
                enums.Add(definition);
        }

        var enumNames = enums.Select(e => e.Name).ToHashSet(StringComparer.Ordinal);
        var commandsByStruct = ScanImplCommands(combined);

        var models = new List<ViewModelDefinition>();
        if (baseline is not null)
            models.AddRange(baseline.Models);

        var views = new List<ViewDefinition>();
        if (baseline is not null)
            views.AddRange(baseline.Views);

        // First pass: collect all scanned model struct names so nested model references resolve regardless of declaration order.
        var knownModelNames = models.Select(m => m.Name).ToHashSet(StringComparer.Ordinal);
        foreach (Match structMatch in StructBlockRegex.Matches(combined))
        {
            var attrs = ParseAttributes(structMatch.Groups["attr"].Value);
            var structName = structMatch.Groups["name"].Value;
            var modelName = GetAttr(attrs, "name") ?? EnsureViewModelSuffix(structName);
            knownModelNames.Add(modelName);
        }

        foreach (Match structMatch in StructBlockRegex.Matches(combined))
        {
            var attrText = structMatch.Groups["attr"].Value;
            var structName = structMatch.Groups["name"].Value;
            var body = structMatch.Groups["body"].Value;
            var attrs = ParseAttributes(attrText);

            var modelName = GetAttr(attrs, "name") ?? EnsureViewModelSuffix(structName);
            var existingModel = models.FirstOrDefault(m => m.Name == modelName);
            var ns = GetAttr(attrs, "namespace")
                ?? existingModel?.ManagedNamespace
                ?? "Avalonia.Rust.Generated";
            var modelId = GetIntAttr(attrs, "id")
                ?? existingModel?.Id
                ?? NextId(models.Select(m => m.Id));

            var properties = new List<ViewModelProperty>();
            var collections = new List<ViewModelCollection>();
            if (existingModel is not null)
            {
                properties.AddRange(existingModel.Properties);
                collections.AddRange(existingModel.Collections);
            }

            ScanFields(body, enumNames, knownModelNames, properties, collections);

            var commands = new List<ViewModelCommand>();
            if (existingModel is not null)
                commands.AddRange(existingModel.Commands);

            if (commandsByStruct.TryGetValue(structName, out var scannedCommands))
            {
                foreach (var cmd in scannedCommands)
                {
                    var existingCmd = commands.FirstOrDefault(c => c.Name == cmd.Name);
                    var cmdId = cmd.ExplicitId
                        ?? existingCmd?.Id
                        ?? NextId(commands.Select(c => c.Id));
                    var built = new ViewModelCommand
                    {
                        Id = cmdId,
                        Name = cmd.Name,
                        IsAsync = cmd.IsAsync,
                        ParameterProperty = cmd.ParameterProperty ?? existingCmd?.ParameterProperty,
                        ResultModelName = cmd.ResultModelName ?? existingCmd?.ResultModelName,
                        SupportsProgress = cmd.SupportsProgress || (existingCmd?.SupportsProgress ?? false),
                        SupportsCancellation = cmd.SupportsCancellation || (existingCmd?.SupportsCancellation ?? false),
                    };
                    var idx = commands.FindIndex(c => c.Name == cmd.Name);
                    if (idx >= 0)
                        commands[idx] = built;
                    else
                        commands.Add(built);
                }
            }

            var modelDef = new ViewModelDefinition
            {
                Id = modelId,
                Name = modelName,
                ManagedNamespace = ns,
                Properties = properties,
                Collections = collections,
                Maps = existingModel?.Maps ?? [],
                Commands = commands,
                Menus = existingModel?.Menus ?? [],
                RecentFiles = existingModel?.RecentFiles,
                DisplayPath = GetAttr(attrs, "display_path") ?? existingModel?.DisplayPath,
                ThemeVariantProperty = GetAttr(attrs, "theme_variant_property") ?? existingModel?.ThemeVariantProperty,
            };

            var existingModelIndex = models.FindIndex(m => m.Name == modelName);
            if (existingModelIndex >= 0)
                models[existingModelIndex] = modelDef;
            else
                models.Add(modelDef);

            if (GetAttr(attrs, "view") is { } viewName)
            {
                var existingView = views.FirstOrDefault(v => v.Name == viewName);
                var viewId = GetIntAttr(attrs, "view_id")
                    ?? existingView?.Id
                    ?? NextId(views.Select(v => v.Id));
                var viewType = GetAttr(attrs, "view_type")
                    ?? existingView?.ManagedTypeName
                    ?? $"{ns}.{viewName}";
                var dynamicBindings = HasFlag(attrs, "dynamic_bindings")
                    || (existingView?.DynamicBindings ?? false);
                var viewDef = new ViewDefinition
                {
                    Id = viewId,
                    Name = viewName,
                    Model = modelName,
                    ManagedTypeName = viewType,
                    DynamicBindings = dynamicBindings,
                    Converters = existingView?.Converters ?? [],
                };
                var existingViewIndex = views.FindIndex(v => v.Name == viewName);
                if (existingViewIndex >= 0)
                    views[existingViewIndex] = viewDef;
                else
                    views.Add(viewDef);
            }
        }

        var ir = new ViewModelIr
        {
            Version = baseline?.Version ?? ViewModelIr.CurrentVersion,
            Enums = enums,
            Models = models,
            Views = views,
            Converters = baseline?.Converters ?? [],
        };
        ir.Validate();
        return ir;
    }

    private static void ScanFields(
        string body,
        HashSet<string> enumNames,
        HashSet<string> knownModelNames,
        List<ViewModelProperty> properties,
        List<ViewModelCollection> collections)
    {
        foreach (Match fieldMatch in FieldRegex.Matches(body))
        {
            var attrKind = fieldMatch.Groups["kind"].Value;
            var attrArgs = ParseAttributes(fieldMatch.Groups["attr"].Value);
            var fieldName = fieldMatch.Groups["name"].Value;
            var rustType = fieldMatch.Groups["type"].Value.Trim();

            var memberName = GetAttr(attrArgs, "name") ?? ToPascalCase(fieldName);

            if (attrKind == "collection" || rustType.StartsWith("Vec<", StringComparison.Ordinal))
            {
                var innerType = ExtractGenericArg(rustType, "Vec") ?? rustType;
                var elementModel = GetAttr(attrArgs, "model");
                ViewModelValueKind elementKind;
                if (elementModel is not null)
                {
                    elementKind = ViewModelValueKind.Model;
                }
                else if (innerType is "String" or "&str")
                {
                    elementKind = ViewModelValueKind.String;
                }
                else if (innerType is "i64" or "i32" or "isize")
                {
                    elementKind = ViewModelValueKind.Integer;
                }
                else if (innerType is "f64" or "f32")
                {
                    elementKind = ViewModelValueKind.Double;
                }
                else
                {
                    elementKind = ViewModelValueKind.Model;
                    elementModel = EnsureViewModelSuffix(innerType);
                }

                var existingCol = collections.FirstOrDefault(c => c.Name == memberName);
                var colId = GetIntAttr(attrArgs, "id")
                    ?? existingCol?.Id
                    ?? NextId(collections.Select(c => c.Id));

                var colDef = new ViewModelCollection
                {
                    Id = colId,
                    Name = memberName,
                    ElementKind = elementKind,
                    ElementModelName = elementModel ?? existingCol?.ElementModelName,
                    Table = existingCol?.Table,
                    Window = existingCol?.Window,
                    Recursive = HasFlag(attrArgs, "recursive") || (existingCol?.Recursive ?? false),
                    Tree = existingCol?.Tree,
                };
                var idx = collections.FindIndex(c => c.Name == memberName);
                if (idx >= 0)
                    collections[idx] = colDef;
                else
                    collections.Add(colDef);
                continue;
            }

            var isOption = ExtractGenericArg(rustType, "Option") is { } optInner;
            var coreType = ExtractGenericArg(rustType, "Option") ?? rustType;
            var nullable = isOption || HasFlag(attrArgs, "nullable");
            var writable = attrKind != "derived" && HasFlag(attrArgs, "writable");

            var explicitEnum = GetAttr(attrArgs, "enum");
            var explicitModel = GetAttr(attrArgs, "model");

            ViewModelValueKind propKind;
            string? enumName = null;
            string? nestedModelName = null;

            if (explicitModel is not null)
            {
                propKind = ViewModelValueKind.Model;
                nestedModelName = explicitModel;
                nullable = true;
                writable = false;
            }
            else if (explicitEnum is not null || enumNames.Contains(coreType))
            {
                propKind = ViewModelValueKind.Enum;
                enumName = explicitEnum ?? coreType;
            }
            else if (coreType is "String" or "&str")
            {
                propKind = ViewModelValueKind.String;
            }
            else if (coreType is "i64" or "i32" or "isize")
            {
                propKind = ViewModelValueKind.Integer;
            }
            else if (coreType == "bool")
            {
                propKind = ViewModelValueKind.Boolean;
            }
            else if (coreType is "f64" or "f32")
            {
                propKind = ViewModelValueKind.Double;
            }
            else if (knownModelNames.Contains(coreType) || knownModelNames.Contains(EnsureViewModelSuffix(coreType)))
            {
                propKind = ViewModelValueKind.Model;
                nestedModelName = knownModelNames.Contains(coreType) ? coreType : EnsureViewModelSuffix(coreType);
                nullable = true;
                writable = false;
            }
            else
            {
                continue;
            }

            var existingProp = properties.FirstOrDefault(p => p.Name == memberName);
            var propId = GetIntAttr(attrArgs, "id")
                ?? existingProp?.Id
                ?? NextId(properties.Select(p => p.Id));

            var initialRaw = GetAttr(attrArgs, "initial");
            string? initialString = existingProp?.InitialString;
            long? initialInteger = existingProp?.InitialInteger;
            bool? initialBoolean = existingProp?.InitialBoolean;
            double? initialDouble = existingProp?.InitialDouble;

            if (initialRaw is not null)
            {
                switch (propKind)
                {
                    case ViewModelValueKind.String:
                        initialString = initialRaw;
                        break;
                    case ViewModelValueKind.Integer:
                    case ViewModelValueKind.Enum:
                        initialInteger = long.Parse(initialRaw, CultureInfo.InvariantCulture);
                        break;
                    case ViewModelValueKind.Boolean:
                        initialBoolean = bool.Parse(initialRaw);
                        break;
                    case ViewModelValueKind.Double:
                        initialDouble = double.Parse(initialRaw, CultureInfo.InvariantCulture);
                        break;
                }
            }

            var propDef = new ViewModelProperty
            {
                Id = propId,
                Name = memberName,
                Kind = propKind,
                Writable = writable,
                Nullable = nullable,
                EnumName = enumName,
                ModelName = nestedModelName,
                InitialString = initialString,
                InitialInteger = initialInteger,
                InitialBoolean = initialBoolean,
                InitialDouble = initialDouble,
            };

            var propIdx = properties.FindIndex(p => p.Name == memberName);
            if (propIdx >= 0)
                properties[propIdx] = propDef;
            else
                properties.Add(propDef);
        }
    }

    private sealed record ScannedCommand(
        int? ExplicitId,
        string Name,
        bool IsAsync,
        string? ParameterProperty,
        string? ResultModelName,
        bool SupportsProgress,
        bool SupportsCancellation);

    private static Dictionary<string, List<ScannedCommand>> ScanImplCommands(string source)
    {
        var result = new Dictionary<string, List<ScannedCommand>>(StringComparer.Ordinal);
        foreach (Match implMatch in ImplBlockRegex.Matches(source))
        {
            var structName = implMatch.Groups["name"].Value;
            var body = implMatch.Groups["body"].Value;
            if (!result.TryGetValue(structName, out var list))
            {
                list = [];
                result[structName] = list;
            }

            foreach (Match cmdMatch in CommandFnRegex.Matches(body))
            {
                var attrs = ParseAttributes(cmdMatch.Groups["attr"].Value);
                var fnName = cmdMatch.Groups["name"].Value;
                var cmdName = GetAttr(attrs, "name") ?? ToPascalCase(fnName);
                var isAsync = HasFlag(attrs, "async") || HasFlag(attrs, "is_async");
                var supportsProgress = HasFlag(attrs, "progress") || HasFlag(attrs, "supports_progress");
                var supportsCancellation = HasFlag(attrs, "cancellable") || HasFlag(attrs, "supports_cancellation");
                if (supportsProgress || supportsCancellation)
                    isAsync = true;

                list.Add(new ScannedCommand(
                    GetIntAttr(attrs, "id"),
                    cmdName,
                    isAsync,
                    GetAttr(attrs, "parameter_property"),
                    GetAttr(attrs, "result_model"),
                    supportsProgress,
                    supportsCancellation));
            }
        }
        return result;
    }

    private static Dictionary<string, string?> ParseAttributes(string raw)
    {
        var dict = new Dictionary<string, string?>(StringComparer.Ordinal);
        if (string.IsNullOrWhiteSpace(raw))
            return dict;

        foreach (Match m in AttrItemRegex.Matches(raw))
        {
            var key = m.Groups["key"].Value;
            if (m.Groups["str"].Success)
                dict[key] = m.Groups["str"].Value;
            else if (m.Groups["val"].Success)
                dict[key] = m.Groups["val"].Value;
            else
                dict[key] = null;
        }
        return dict;
    }

    private static string? GetAttr(Dictionary<string, string?> attrs, string key) =>
        attrs.TryGetValue(key, out var val) ? val : null;

    private static int? GetIntAttr(Dictionary<string, string?> attrs, string key) =>
        attrs.TryGetValue(key, out var val) && val is not null && int.TryParse(val, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : null;

    private static bool HasFlag(Dictionary<string, string?> attrs, string key) =>
        attrs.TryGetValue(key, out var val) && (val is null || string.Equals(val, "true", StringComparison.OrdinalIgnoreCase));

    private static int NextId(IEnumerable<int> existing)
    {
        var max = 0;
        foreach (var id in existing)
        {
            if (id > max)
                max = id;
        }
        return max + 1;
    }

    private static string EnsureViewModelSuffix(string name) =>
        name.EndsWith("ViewModel", StringComparison.Ordinal) ? name : name + "ViewModel";

    private static string? ExtractGenericArg(string rustType, string wrapper)
    {
        var prefix = wrapper + "<";
        if (rustType.StartsWith(prefix, StringComparison.Ordinal) && rustType.EndsWith(">", StringComparison.Ordinal))
            return rustType[prefix.Length..^1].Trim();
        return null;
    }

    private static string ToPascalCase(string snake)
    {
        var parts = snake.Split('_', StringSplitOptions.RemoveEmptyEntries);
        return string.Concat(parts.Select(part => char.ToUpperInvariant(part[0]) + part[1..]));
    }

    private static readonly Regex EnumBlockRegex = new(
        @"#\[(?:rustolonia::)?view_model_enum(?:\((?<attr>(?:""[^""]*""|[^)])*)\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?enum\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\{(?<body>[^}]*)\}",
        RegexOptions.Compiled | RegexOptions.Singleline);

    private static readonly Regex EnumMemberRegex = new(
        @"(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*(?<value>-?[0-9]+))?\s*,?",
        RegexOptions.Compiled);

    private static readonly Regex StructBlockRegex = new(
        @"#\[(?:rustolonia::)?view_model(?:\((?<attr>(?:""[^""]*""|[^)])*)\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?struct\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\{(?<body>[^}]*)\}",
        RegexOptions.Compiled | RegexOptions.Singleline);

    private static readonly Regex FieldRegex = new(
        @"#\[(?<kind>prop|derived|collection)(?:\((?<attr>(?:""[^""]*""|[^)])*)\))?\]\s*(?:pub(?:\([^)]*\))?\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,\n}]+)",
        RegexOptions.Compiled);

    private static readonly Regex ImplBlockRegex = new(
        @"impl\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\{(?<body>(?:[^{}]|\{[^{}]*\})*)\}",
        RegexOptions.Compiled | RegexOptions.Singleline);

    private static readonly Regex CommandFnRegex = new(
        @"#\[command(?:\((?<attr>(?:""[^""]*""|[^)])*)\))?\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)",
        RegexOptions.Compiled);

    private static readonly Regex AttrItemRegex = new(
        @"(?<key>[A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*(?:""(?<str>[^""]*)""|(?<val>[^,\s)]+)))?",
        RegexOptions.Compiled);
}
