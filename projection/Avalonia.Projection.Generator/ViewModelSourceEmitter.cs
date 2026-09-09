using System.Globalization;
using System.Text;
using System.Text.Json;
using Avalonia.Projection.Ir;

namespace Avalonia.Projection.Generator;

public static class ViewModelSourceEmitter
{
    public static IReadOnlyDictionary<string, string> EmitCSharp(ViewModelIr ir)
    {
        ir.Validate();
        var files = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        void Add(string name, string content)
        {
            if (!files.TryAdd(name, content))
                throw new InvalidOperationException($"Duplicate generated output '{name}'.");
        }
        Add("RustViewRegistry.g.cs", EmitRegistry(ir));
        foreach (var enumDefinition in ir.Enums)
            Add($"{enumDefinition.Name}.g.cs", EmitCSharpEnum(enumDefinition));
        foreach (var model in ir.Models)
        {
            Add($"{model.Name}Adapter.g.cs", EmitCSharpAdapter(ir, model));
            Add($"{model.Name}Metadata.g.cs", EmitCSharpMetadata(ir, model));
            if (model.Menus.Count > 0)
                Add($"{model.Name}Menus.g.cs", EmitCSharpMenus(ir, model));
        }
        foreach (var converter in ir.Converters)
            Add($"{converter.Name}Converter.g.cs", EmitCSharpConverter(converter));
        return files;
    }

    public static string EmitRust(ViewModelIr ir, bool externalConsumer = false)
    {
        ir.Validate();
        var sb = new StringBuilder(
            "//! Generated from view-model.ir.json. Do not edit.\n\n");
        if (externalConsumer)
            sb.AppendLine("#![allow(dead_code)]\n");
        foreach (var enumDefinition in ir.Enums)
            EmitRustEnum(sb, enumDefinition);
        foreach (var model in ir.Models)
            EmitRustModel(sb, ir, model, ir.Views.Where(view => view.Model == model.Name), externalConsumer);
        EmitRustConverters(sb, ir.Converters, externalConsumer);
        return sb.ToString().TrimEnd() + "\n";
    }

    public static string EmitContract(ViewModelIr ir)
    {
        ir.Validate();
        var sb = new StringBuilder();
        sb.AppendLine("# Generated Rust view-model contract");
        sb.AppendLine();
        sb.AppendLine($"Schema version: `{ir.Version}`");
        if (ir.Enums.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("## Enums");
            sb.AppendLine();
            sb.AppendLine("| ID | Name | Members |");
            sb.AppendLine("| ---: | --- | --- |");
            foreach (var enumDefinition in ir.Enums)
            {
                var members = string.Join(", ", enumDefinition.Members.Select(member => $"`{member.Name}` = {member.Value}"));
                sb.AppendLine($"| {enumDefinition.Id} | `{enumDefinition.Name}` | {members} |");
            }
        }
        foreach (var model in ir.Models)
        {
            sb.AppendLine();
            sb.AppendLine($"## Model `{model.Name}` (`{model.Id}`)");
            sb.AppendLine();
            sb.AppendLine("| Kind | ID | Name | Type | Direction |");
            sb.AppendLine("| --- | ---: | --- | --- | --- |");
            foreach (var property in model.Properties)
            {
                var type = property.Kind switch
                {
                    ViewModelValueKind.Enum => $"Enum `{property.EnumName}`",
                    ViewModelValueKind.Model => $"Model `{property.ModelName}`",
                    _ => $"`{property.Kind}`",
                };
                var nullable = property.Nullable ? ", nullable" : "";
                var direction = property.Kind == ViewModelValueKind.Model
                    ? "Rust to managed"
                    : property.Writable ? "Rust and managed" : "Rust to managed";
                sb.AppendLine($"| Property | {property.Id} | `{property.Name}` | {type}{nullable} | {direction} |");
            }
            foreach (var collection in model.Collections)
            {
                var type = collection.ElementKind == ViewModelValueKind.Model
                    ? $"Model `{collection.ElementModelName}`"
                    : $"`{collection.ElementKind}`";
                var shape = collection.Window is { } window
                    ? $" (windowed: page {window.PageSize}, {window.MaxLivePages} live pages)"
                    : collection.Tree is not null ? " (tree root)"
                    : collection.Recursive ? " (recursive children)" : "";
                sb.AppendLine($"| Collection | {collection.Id} | `{collection.Name}` | {type}{shape} | Rust to managed |");
            }
            foreach (var map in model.Maps)
            {
                var value = map.ValueKind == ViewModelValueKind.Model
                    ? $"Model `{map.ValueModelName}`"
                    : $"`{map.ValueKind}`";
                sb.AppendLine($"| Map | {map.Id} | `{map.Name}` | `{map.KeyKind}` to {value} | Rust to managed |");
            }
            foreach (var command in model.Commands)
            {
                var parameter = command.ParameterProperty is null
                    ? "None"
                    : $"`{command.ParameterProperty}`";
                var extras = new List<string>();
                if (command.ResultModelName is not null)
                    extras.Add($"result `{command.ResultModelName}`");
                if (command.SupportsProgress)
                    extras.Add("progress");
                if (command.SupportsCancellation)
                    extras.Add("cancellable");
                var suffix = extras.Count == 0 ? "" : $" ({string.Join(", ", extras)})";
                sb.AppendLine($"| {(command.IsAsync ? "Async command" : "Command")} | {command.Id} | `{command.Name}`{suffix} | {parameter} | Managed to Rust |");
            }
            foreach (var collection in model.Collections.Where(collection => collection.Tree is not null))
            {
                var tree = collection.Tree!;
                sb.AppendLine();
                sb.AppendLine($"### Tree `{collection.Name}`");
                sb.AppendLine();
                sb.AppendLine($"Node model `{collection.ElementModelName}`, children `{tree.ChildrenCollection}`, header `{tree.HeaderPath}`, has-children `{tree.HasChildrenProperty ?? "-"}`.");
            }
            if (model.DisplayPath is { } displayPath)
            {
                sb.AppendLine();
                sb.AppendLine($"Display projection (`ToString()`): `{displayPath}`.");
            }
            if (model.RecentFiles is { } recent)
            {
                sb.AppendLine();
                sb.AppendLine($"### Recent files `{recent.Collection}`");
                sb.AppendLine();
                sb.AppendLine(
                    $"Storage URIs published into collection `{recent.Collection}`, capacity {recent.Capacity}, activated by `{recent.ActivateCommand}Command` with the chosen URI as its command parameter.");
            }
            foreach (var menu in model.Menus)
            {
                sb.AppendLine();
                sb.AppendLine($"### {menu.Kind} menu `{menu.Name}` (`{menu.Id}`)");
                sb.AppendLine();
                sb.AppendLine("| ID | Item | Kind | Header | Command | Gesture | Bound member |");
                sb.AppendLine("| ---: | --- | --- | --- | --- | --- | --- |");
                AppendMenuRows(sb, menu.Items, 0);
            }
            foreach (var collection in model.Collections.Where(collection => collection.Table is not null))
            {
                var table = collection.Table!;
                sb.AppendLine();
                sb.AppendLine($"### Table `{collection.Name}`");
                sb.AppendLine();
                sb.AppendLine("| ID | Name | Header | Row path | Width | Resize | Sort | Alignment |");
                sb.AppendLine("| ---: | --- | --- | --- | --- | --- | --- | --- |");
                foreach (var column in table.Columns)
                {
                    var width = column.Auto ? "Auto" : column.Star ? "*" : column.Width!.Value.ToString(CultureInfo.InvariantCulture);
                    sb.AppendLine($"| {column.Id} | `{column.Name}` | {column.Header} | `{column.Path}` | {width} | {(column.Resizable ? "Yes" : "No")} | {(column.Sortable ? "Yes" : "No")} | {column.HorizontalAlignment} |");
                }
                if (table.Selection is { } selection)
                    sb.AppendLine($"Selection: index `{selection.SelectedIndexProperty ?? "-"}`, key `{selection.SelectedKeyProperty ?? "-"}`, row key `{selection.RowKeyPath ?? "-"}`.");
                if (table.Sort is { } sort)
                    sb.AppendLine($"Sort: `{sort.Command}` command, initial column `{sort.Column}`, direction property `{sort.DirectionProperty}`.");
            }
        }
        if (ir.Converters.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("## Value converters");
            sb.AppendLine();
            sb.AppendLine("| ID | Name | Value | Parameter | Result | ConvertBack | Used by |");
            sb.AppendLine("| ---: | --- | --- | --- | --- | --- | --- |");
            foreach (var converter in ir.Converters)
            {
                var parameter = converter.ParameterKind is { } parameterKind ? $"`{parameterKind}`" : "None";
                var usedBy = ir.Views.Where(view => view.Converters.Contains(converter.Name)).Select(view => view.Name).ToArray();
                var usedByText = usedBy.Length == 0 ? "-" : string.Join(", ", usedBy.Select(name => $"`{name}`"));
                sb.AppendLine(
                    $"| {converter.Id} | `{converter.Name}` | `{converter.ValueKind}` | {parameter} | `{converter.ResultKind}` | {(converter.SupportsConvertBack ? "Yes" : "No")} | {usedByText} |");
            }
        }
        if (ir.Views.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("## Views");
            sb.AppendLine();
            sb.AppendLine("| ID | Name | Model | Managed type | Binding path |");
            sb.AppendLine("| ---: | --- | --- | --- | --- |");
            foreach (var view in ir.Views)
                sb.AppendLine(
                    $"| {view.Id} | `{view.Name}` | `{view.Model}` | `{view.ManagedTypeName}` | {(view.DynamicBindings ? "Dynamic Rust metadata" : "Generated CLR properties")} |");
        }
        return sb.ToString();
    }

    /// <summary>
    /// Renders one menu's items, indenting nested submenu items so the tree
    /// shape survives in the flat contract table.
    /// </summary>
    private static void AppendMenuRows(StringBuilder sb, IReadOnlyList<ViewModelMenuItem> items, int depth)
    {
        var prefix = depth == 0 ? "" : new string('\u00a0', depth * 4);
        foreach (var item in items)
        {
            var bound = item.Kind switch
            {
                ViewModelMenuItemKind.Toggle => $"`{item.ToggleProperty}`",
                ViewModelMenuItemKind.Radio => $"`{item.RadioProperty}` = `{item.RadioValue}`",
                ViewModelMenuItemKind.RecentFiles => "recent files",
                _ => item.IsEnabledProperty is { } enabled ? $"enabled by `{enabled}`" : "-",
            };
            var command = item.Command is { } name ? $"`{name}Command`" : "-";
            var gesture = item.Gesture is { } gesture1 ? $"`{gesture1}`" : "-";
            var header = item.Header is { } text ? text : "-";
            sb.AppendLine($"| {item.Id} | {prefix}`{item.Name}` | {item.Kind} | {header} | {command} | {gesture} | {bound} |");
            if (item.Items.Count > 0)
                AppendMenuRows(sb, item.Items, depth + 1);
        }
    }

    private static string EmitCSharpAdapter(ViewModelIr ir, ViewModelDefinition model)
    {        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine("using System;");
        sb.AppendLine("using System.Collections;");
        sb.AppendLine("using System.Collections.Generic;");
        sb.AppendLine("using System.Collections.ObjectModel;");
        sb.AppendLine("using System.ComponentModel;");
        sb.AppendLine("using System.Globalization;");
        sb.AppendLine("using System.Linq;");
        sb.AppendLine("using System.Runtime.CompilerServices;");
        sb.AppendLine("using System.Runtime.InteropServices;");
        sb.AppendLine("using System.Runtime.InteropServices.Marshalling;");
        sb.AppendLine("using System.Windows.Input;");
        if (model.ThemeVariantProperty is not null)
        {
            sb.AppendLine("using Avalonia;");
            sb.AppendLine("using Avalonia.Styling;");
        }
        sb.AppendLine("using Avalonia.Rust;");
        sb.AppendLine("using Avalonia.Rust.Interop;");
        sb.AppendLine("using Avalonia.Threading;");
        sb.AppendLine();
        sb.AppendLine($"namespace {model.ManagedNamespace};");
        sb.AppendLine();
        var maps = model.Maps;
        var windowedCollections = model.Collections.Where(collection => collection.Window is not null).ToArray();
        var materializedCollections = model.Collections.Where(collection => collection.Window is null).ToArray();
        var resultCommands = model.Commands.Where(command => command.ResultModelName is not null).ToArray();
        var progressCommands = model.Commands.Where(command => command.SupportsProgress).ToArray();
        var cancellableCommands = model.Commands.Where(command => command.SupportsCancellation).ToArray();
        var trackedCommands = model.Commands.Where(command => command.SupportsCancellation || command.SupportsProgress).ToArray();
        var shapes = maps.Count > 0 || windowedCollections.Length > 0 ||
            resultCommands.Length > 0 || progressCommands.Length > 0 || cancellableCommands.Length > 0;
        // A scalar-number collection is the only shape that needs the v5 sink,
        // so a model without one keeps exactly the interface set it already
        // published.
        var numberCollections = materializedCollections.Where(IsNumberCollection).ToArray();
        sb.AppendLine("[GeneratedComClass]");
        var interfaces = "IAvnRustVmSink, IAvnRustVmSink2, IAvnRustVmSink3, " +
            (shapes ? "IAvnRustVmSink4, " : "") +
            (numberCollections.Length > 0 ? "IAvnRustVmSink5, " : "") +
            "IRustVmStringSnapshotSink, IRustVmModelSnapshotSink, IRustVmBatchTarget, IRustVmTableSelectionBatchTarget, INotifyPropertyChanged, INotifyDataErrorInfo, IDisposable";
        sb.AppendLine($"public sealed partial class {model.Name}Adapter : {interfaces}");
        sb.AppendLine("{");
        sb.AppendLine("    private readonly IAvnRustViewModel _model;");
        sb.AppendLine("    private readonly Action<Action> _dispatch;");
        sb.AppendLine("    private readonly Action<Action>? _post;");
        sb.AppendLine("    private readonly RustVmBatchCoordinator _batch;");
        sb.AppendLine("    private readonly Dictionary<string, string> _errors = new(StringComparer.Ordinal);");
        sb.AppendLine("    private readonly RustVmInboundWriteTracker _inboundWrites = new();");
        if (trackedCommands.Length > 0)
            sb.AppendLine("    private readonly IAvnRustViewModel2? _tracked;");
        if (windowedCollections.Length > 0)
            sb.AppendLine("    private readonly RustRangeCoordinator _ranges;");
        foreach (var command in trackedCommands)
            sb.AppendLine($"    private long _{Lower(command.Name)}Operation;");
        foreach (var property in model.Properties)
            sb.AppendLine($"    private {CSharpType(ir, property)} _{Lower(property.Name)} = {CSharpInitial(ir, property)};");
        foreach (var command in resultCommands)
            sb.AppendLine($"    private {CSharpModelAdapterTypeName(ir, command.ResultModelName!)}? _{Lower(command.Name)}Result;");
        foreach (var command in progressCommands)
        {
            sb.AppendLine($"    private double? _{Lower(command.Name)}Progress;");
            sb.AppendLine($"    private string? _{Lower(command.Name)}ProgressMessage;");
        }
        foreach (var command in trackedCommands)
            sb.AppendLine($"    private bool _{Lower(command.Name)}IsRunning;");
        sb.AppendLine();
        sb.AppendLine("    /// <summary>Creates an adapter that dispatches and posts through <see cref=\"Dispatcher.UIThread\"/>.</summary>");
        sb.AppendLine($"    public {model.Name}Adapter(IAvnRustViewModel model) : this(model, null, null) {{ }}");
        sb.AppendLine();
        sb.AppendLine("    /// <summary>");
        sb.AppendLine("    /// Creates an adapter with a custom synchronous dispatch for the legacy v1/v2");
        sb.AppendLine("    /// sink path. Kept as a distinct CLR signature (not an optional parameter) so");
        sb.AppendLine("    /// already-compiled callers keep binding to it.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public {model.Name}Adapter(IAvnRustViewModel model, Action<Action>? dispatch) : this(model, dispatch, null) {{ }}");
        sb.AppendLine();
        sb.AppendLine("    /// <summary>");
        sb.AppendLine("    /// Creates an adapter with a custom synchronous <paramref name=\"dispatch\"/> for the");
        sb.AppendLine("    /// legacy v1/v2 sink path and a custom nonblocking <paramref name=\"post\"/> for");
        sb.AppendLine("    /// batch submission.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public {model.Name}Adapter(IAvnRustViewModel model, Action<Action>? dispatch, Action<Action>? post)");
        sb.AppendLine("    {");
        sb.AppendLine("        _model = model;");
        sb.AppendLine("        _dispatch = dispatch ?? Dispatch;");
        sb.AppendLine("        _post = post;");
        sb.AppendLine("        _batch = new RustVmBatchCoordinator(this, post);");
        if (trackedCommands.Length > 0)
            sb.AppendLine("        _tracked = RustAsyncCommands.TryResolve(model);");
        if (windowedCollections.Length > 0)
        {
            sb.AppendLine("        _ranges = new RustRangeCoordinator(ResolveWindow, post);");
            sb.AppendLine("        var rangeSource = RustAsyncCommands.TryResolveRangeSource(model);");
            foreach (var collection in windowedCollections)
            {
                var window = collection.Window!;
                var elementType = CSharpElementType(ir, collection.ElementKind, collection.ElementModelName);
                var factory = collection.ElementKind == ViewModelValueKind.Model
                    ? $"(nested, _) => new {elementType}(nested!, _dispatch, _post)"
                    : "(_, text) => text ?? string.Empty";
                sb.AppendLine($"        {collection.Name} = new RustWindowedCollection({collection.Id}, {window.PageSize}, {window.MaxLivePages}, {factory});");
                sb.AppendLine($"        {collection.Name}.SetSource(rangeSource);");
                sb.AppendLine($"        {collection.Name}.SetPeerResolver(ResolveWindow);");
            }
        }
        foreach (var command in model.Commands)
        {
            var invocation = command.IsAsync ? "BeginAsync" : "Execute";
            var parameter = CSharpCommandWireArgument(model, command);
            if (command.IsAsync && (command.SupportsCancellation || command.SupportsProgress))
            {
                sb.AppendLine(
                    $"        {command.Name}Command = new DelegateCommand(parameter => _{Lower(command.Name)}Operation = RustAsyncCommands.Begin(_model, _tracked, {command.Id}, {parameter}));");
            }
            else
            {
                sb.AppendLine($"        {command.Name}Command = new DelegateCommand(parameter => Check(_model.{invocation}({command.Id}, {parameter})));");
            }
            if (command.SupportsCancellation)
            {
                sb.AppendLine(
                    $"        Cancel{command.Name}Command = new DelegateCommand(_ => RustAsyncCommands.Cancel(_tracked, {command.Id}, _{Lower(command.Name)}Operation));");
                sb.AppendLine($"        Cancel{command.Name}Command.SetEnabledCore(false);");
            }
        }
        sb.AppendLine("        try");
        sb.AppendLine("        {");
        sb.AppendLine("            Check(_model.Attach(this));");
        if (windowedCollections.Length > 0)
        {
            sb.AppendLine("            // Primed after attach: a producer publishes its dataset identity");
            sb.AppendLine("            // from attach, so reading it before would always come back empty.");
            sb.AppendLine("            PrimeWindows(rangeSource);");
        }
        sb.AppendLine("        }");
        sb.AppendLine("        catch");
        sb.AppendLine("        {");
        sb.AppendLine("            try { _model.Detach(); }");
        sb.AppendLine("            catch { }");
        sb.AppendLine("            DisposeNestedAdapters();");
        sb.AppendLine("            throw;");
        sb.AppendLine("        }");
        if (model.ThemeVariantProperty is { } themeProperty)
        {
            sb.AppendLine("        ApplyThemeVariant();");
            sb.AppendLine($"        PropertyChanged += (_, e) => {{ if (e.PropertyName is null || e.PropertyName == nameof({themeProperty})) ApplyThemeVariant(); }};");
        }
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    public event PropertyChangedEventHandler? PropertyChanged;");
        sb.AppendLine("    public event EventHandler<DataErrorsChangedEventArgs>? ErrorsChanged;");
        sb.AppendLine();
        foreach (var property in model.Properties)
        {
            sb.AppendLine($"    public {CSharpType(ir, property)} {property.Name}");
            sb.AppendLine("    {");
            sb.AppendLine($"        get => _{Lower(property.Name)};");
            if (property.Writable)
            {
                sb.AppendLine("        set");
                sb.AppendLine("        {");
                sb.AppendLine($"            var accepted = {CSharpAcceptedValue(property, "value")};");
                sb.AppendLine($"            if (Equals(_{Lower(property.Name)}, accepted))");
                sb.AppendLine("                return;");
                sb.AppendLine($"            var previous = _{Lower(property.Name)};");
                sb.AppendLine($"            var inbound = _inboundWrites.Begin({property.Id});");
                sb.AppendLine("            try");
                sb.AppendLine("            {");
                sb.AppendLine($"                Check(_model.Set{TransportSuffix(WireKind(property.Kind))}({property.Id}, {CSharpToTransport(property, "accepted")}));");
                sb.AppendLine("                if (!_inboundWrites.WasPublished(inbound))");
                sb.AppendLine("                {");
                sb.AppendLine($"                    _inboundWrites.CommitLocal({property.Id});");
                sb.AppendLine($"                    SetField(ref _{Lower(property.Name)}, accepted, nameof({property.Name}));");
                sb.AppendLine("                }");
                sb.AppendLine("            }");
                sb.AppendLine("            catch");
                sb.AppendLine("            {");
                sb.AppendLine("                if (_inboundWrites.ShouldRollback(inbound))");
                sb.AppendLine("                {");
                sb.AppendLine($"                    _inboundWrites.CommitLocal({property.Id});");
                sb.AppendLine($"                    SetField(ref _{Lower(property.Name)}, previous, nameof({property.Name}));");
                sb.AppendLine("                }");
                sb.AppendLine("                throw;");
                sb.AppendLine("            }");
                sb.AppendLine("            finally { _inboundWrites.End(inbound); }");
                sb.AppendLine("        }");
            }
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        foreach (var collection in materializedCollections)
            sb.AppendLine($"    public BatchObservableCollection<{CSharpElementType(ir, collection.ElementKind, collection.ElementModelName)}> {collection.Name} {{ get; }} = [];");
        foreach (var collection in windowedCollections)
        {
            sb.AppendLine("    /// <summary>");
            sb.AppendLine($"    /// Range-backed projection: <c>Count</c> is the Rust dataset's total size while at");
            sb.AppendLine($"    /// most {collection.Window!.PageSize} x {collection.Window.MaxLivePages} element objects are live.");
            sb.AppendLine("    /// </summary>");
            sb.AppendLine($"    public RustWindowedCollection {collection.Name} {{ get; }}");
        }
        foreach (var map in maps)
            sb.AppendLine($"    public RustObservableMap<{CSharpMapKeyType(map)}, {CSharpMapValueType(ir, map)}> {map.Name} {{ get; }} = new();");
        if (model.Collections.Count > 0 || maps.Count > 0)
            sb.AppendLine();
        foreach (var command in model.Commands)
            sb.AppendLine($"    public DelegateCommand {command.Name}Command {{ get; }}");
        foreach (var command in cancellableCommands)
        {
            sb.AppendLine("    /// <summary>Cancels the in-flight invocation. Disabled while nothing is running.</summary>");
            sb.AppendLine($"    public DelegateCommand Cancel{command.Name}Command {{ get; }}");
        }
        foreach (var command in resultCommands)
        {
            sb.AppendLine($"    /// <summary>The command's last typed structured result, or null.</summary>");
            sb.AppendLine($"    public {CSharpModelAdapterTypeName(ir, command.ResultModelName!)}? {command.Name}Result => _{Lower(command.Name)}Result;");
        }
        foreach (var command in progressCommands)
        {
            sb.AppendLine("    /// <summary>Determinate progress in 0..1, or null while progress is indeterminate.</summary>");
            sb.AppendLine($"    public double? {command.Name}Progress => _{Lower(command.Name)}Progress;");
            sb.AppendLine($"    public string? {command.Name}ProgressMessage => _{Lower(command.Name)}ProgressMessage;");
        }
        foreach (var command in trackedCommands)
            sb.AppendLine($"    public bool {command.Name}IsRunning => _{Lower(command.Name)}IsRunning;");
        EmitCSharpDisplayPath(sb, ir, model);
        sb.AppendLine();
        sb.AppendLine("    public bool HasErrors => _errors.Count > 0;");
        sb.AppendLine();
        sb.AppendLine("    public IEnumerable GetErrors(string? propertyName) =>");
        sb.AppendLine("        propertyName is not null && _errors.TryGetValue(propertyName, out var message)");
        sb.AppendLine("            ? new[] { message }");
        sb.AppendLine("            : Array.Empty<string>();");
        sb.AppendLine();
        EmitCSharpSinkMethods(sb, ir, model);
        if (shapes)
            EmitCSharpShapeSinkMethods(sb, ir, model, maps, windowedCollections, resultCommands, progressCommands, trackedCommands);
        if (numberCollections.Length > 0)
            EmitCSharpNumberSinkMethods(sb, numberCollections);
        sb.AppendLine("    public int ReplaceStringSnapshot(int collectionId, IReadOnlyList<string> values) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in materializedCollections.Where(collection => collection.ElementKind == ViewModelValueKind.String))
            sb.AppendLine($"        {collection.Id} => Apply(() => {collection.Name}.ReplaceSnapshot(values)),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    public int ReplaceModelSnapshot(int collectionId, IReadOnlyList<IAvnRustViewModel> values) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in materializedCollections.Where(collection => collection.ElementKind == ViewModelValueKind.Model))
        {
            var adapter = CSharpModelAdapterTypeName(ir, collection.ElementModelName!);
            sb.AppendLine($"        {collection.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            var staged = new List<{adapter}>();");
            sb.AppendLine("            try { foreach (var value in values) staged.Add(new " + adapter + "(value, _dispatch, _post)); }");
            sb.AppendLine("            catch { foreach (var value in staged) TryDispose(value); throw; }");
            sb.AppendLine($"            var previous = {collection.Name}.ToArray();");
            sb.AppendLine($"            {collection.Name}.ReplaceSnapshot(staged);");
            sb.AppendLine("            foreach (var value in previous) TryDispose(value);");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();
        EmitCSharpBatchTarget(sb, ir, model);
        sb.AppendLine("    /// <summary>");
        sb.AppendLine("    /// Enqueues one immutable batch. This call never reads the batch, applies it,");
        sb.AppendLine("    /// or completes it on the submitting (Rust worker) stack.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine("    public int SubmitBatch(IAvnRustVmUpdateBatch? batch) => _batch.Submit(batch);");
        sb.AppendLine();
        sb.AppendLine("    /// <summary>");
        sb.AppendLine("    /// Detaches the model and disposes every nested adapter exactly once. When a");
        sb.AppendLine("    /// batch notification triggers this re-entrantly, the gate defers the cleanup");
        sb.AppendLine("    /// until the batch's commit and notifications have finished.");
        sb.AppendLine("    /// </summary>");
        if (model.Commands.Any(command => command.ParameterProperty is not null))
        {
            sb.AppendLine("    private static string CommandArgumentOrFallback(object? parameter, string fallback)");
            sb.AppendLine("    {");
            sb.AppendLine("        if (parameter is null)");
            sb.AppendLine("            return fallback;");
            sb.AppendLine("        return parameter switch");
            sb.AppendLine("        {");
            sb.AppendLine("            string text => text,");
            sb.AppendLine("            bool flag => flag ? \"true\" : \"false\",");
            sb.AppendLine("            IFormattable formattable => formattable.ToString(null, CultureInfo.InvariantCulture) ?? fallback,");
            sb.AppendLine("            _ => parameter.ToString() ?? fallback,");
            sb.AppendLine("        };");
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        sb.AppendLine("    public void Dispose() => _batch.Dispose(DisposeCore);");
        sb.AppendLine();
        if (model.ThemeVariantProperty is { } themeName)
        {
            sb.AppendLine("    private void ApplyThemeVariant()");
            sb.AppendLine("    {");
            sb.AppendLine("        if (Application.Current is { } application)");
            sb.AppendLine($"            application.RequestedThemeVariant = _{Lower(themeName)} ? ThemeVariant.Dark : ThemeVariant.Light;");
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        sb.AppendLine("    private void DisposeCore()");
        sb.AppendLine("    {");
        if (windowedCollections.Length > 0)
            sb.AppendLine("        _ranges.Close();");
        sb.AppendLine("        try");
        sb.AppendLine("        {");
        sb.AppendLine("            Check(_model.Detach());");
        sb.AppendLine("        }");
        sb.AppendLine("        finally");
        sb.AppendLine("        {");
        sb.AppendLine("            DisposeNestedAdapters();");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine();
        if (windowedCollections.Length > 0)
        {
            sb.AppendLine("    private RustWindowedCollection? ResolveWindow(int collectionId) => collectionId switch");
            sb.AppendLine("    {");
            foreach (var collection in windowedCollections)
                sb.AppendLine($"        {collection.Id} => {collection.Name},");
            sb.AppendLine("        _ => null,");
            sb.AppendLine("    };");
            sb.AppendLine();
            sb.AppendLine("    /// <summary>");
            sb.AppendLine("    /// Reads each window's dataset identity once, so the first frame already");
            sb.AppendLine("    /// reports the real total count instead of an empty list. Reading it is a");
            sb.AppendLine("    /// lock-free producer-side lookup; it never enters application model code.");
            sb.AppendLine("    /// </summary>");
            sb.AppendLine("    private void PrimeWindows(IAvnRustRangeSource? source)");
            sb.AppendLine("    {");
            sb.AppendLine("        if (source is null) return;");
            foreach (var collection in windowedCollections)
            {
                sb.AppendLine($"        if (source.GetRangeState({collection.Id}, out var generation{collection.Id}, out var total{collection.Id}) >= 0)");
                sb.AppendLine($"            {collection.Name}.ResetTo(generation{collection.Id}, total{collection.Id});");
            }
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        sb.AppendLine("    private void DisposeNestedAdapters()");
        sb.AppendLine("    {");
        foreach (var property in model.Properties.Where(property => property.Kind == ViewModelValueKind.Model))
            sb.AppendLine($"        TryDispose(_{Lower(property.Name)});");
        foreach (var collection in materializedCollections.Where(collection => collection.ElementKind == ViewModelValueKind.Model))
            sb.AppendLine($"        foreach (var item in {collection.Name}) TryDispose(item);");
        foreach (var collection in windowedCollections)
            sb.AppendLine($"        TryDispose({collection.Name});");
        foreach (var map in maps.Where(map => map.ValueKind == ViewModelValueKind.Model))
            sb.AppendLine($"        foreach (var entry in {map.Name}) TryDispose(entry.Value);");
        foreach (var command in resultCommands)
            sb.AppendLine($"        TryDispose(_{Lower(command.Name)}Result);");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private static void TryDispose(IDisposable? value)");
        sb.AppendLine("    {");
        sb.AppendLine("        try { value?.Dispose(); }");
        sb.AppendLine("        catch { }");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private int Apply(Action action) => Apply(() =>");
        sb.AppendLine("    {");
        sb.AppendLine("        action();");
        sb.AppendLine("        return 0;");
        sb.AppendLine("    });");
        sb.AppendLine();
        sb.AppendLine("    private int Apply(Func<int> action)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (_batch.IsClosed) return 0;");
        sb.AppendLine("        var hresult = 0;");
        sb.AppendLine("        void ApplyIfAlive()");
        sb.AppendLine("        {");
        sb.AppendLine("            if (!_batch.IsClosed)");
        sb.AppendLine("            {");
        sb.AppendLine("                try { hresult = action(); }");
        sb.AppendLine("                catch { hresult = unchecked((int)0x80004005); }");
        sb.AppendLine("            }");
        sb.AppendLine("        }");
        sb.AppendLine("        try { _dispatch(ApplyIfAlive); }");
        sb.AppendLine("        catch { return unchecked((int)0x80004005); }");
        sb.AppendLine("        return hresult;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private static void Dispatch(Action action)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (Dispatcher.UIThread.CheckAccess()) action();");
        sb.AppendLine("        else Dispatcher.UIThread.Invoke(action);");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private static void Check(int hresult)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (hresult < 0) Marshal.ThrowExceptionForHR(hresult);");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private void SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (Equals(field, value)) return;");
        sb.AppendLine("        field = value;");
        sb.AppendLine("        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private void SetError(string propertyName, string? message)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (RustVmBatchErrors.Set(_errors, propertyName, message))");
        sb.AppendLine("            ErrorsChanged?.Invoke(this, new DataErrorsChangedEventArgs(propertyName));");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    public sealed class DelegateCommand(Action<object?> execute) : ICommand, IRustVmBatchCommand");
        sb.AppendLine("    {");
        sb.AppendLine("        private bool _canExecute = true;");
        sb.AppendLine();
        sb.AppendLine("        public DelegateCommand(Action execute) : this(_ => execute()) { }");
        sb.AppendLine();
        sb.AppendLine("        public event EventHandler? CanExecuteChanged;");
        sb.AppendLine("        public bool CanExecute(object? parameter) => _canExecute;");
        sb.AppendLine("        public void Execute(object? parameter) => execute(parameter);");
        sb.AppendLine();
        sb.AppendLine("        public void SetEnabled(bool value)");
        sb.AppendLine("        {");
        sb.AppendLine("            if (SetEnabledCore(value)) RaiseCanExecuteChanged();");
        sb.AppendLine("        }");
        sb.AppendLine();
        sb.AppendLine("        public bool SetEnabledCore(bool enabled)");
        sb.AppendLine("        {");
        sb.AppendLine("            if (_canExecute == enabled) return false;");
        sb.AppendLine("            _canExecute = enabled;");
        sb.AppendLine("            return true;");
        sb.AppendLine("        }");
        sb.AppendLine();
        sb.AppendLine("        public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        return sb.ToString();
    }

    /// <summary>
    /// Emits the adapter's <c>IRustVmBatchTarget</c> implementation: the schema
    /// the shared staged engine validates against, the nested-adapter factories
    /// it stages with, and notification-free stores it commits through. The
    /// engine owns all ordering, atomicity and notification coalescing, so the
    /// generated code here is pure switch-over-id with no transactional logic.
    /// </summary>
    private static void EmitCSharpBatchTarget(StringBuilder sb, ViewModelIr ir, ViewModelDefinition model)
    {
        var modelProperties = model.Properties.Where(property => property.Kind == ViewModelValueKind.Model).ToArray();
        // A windowed collection has no materialized managed list, so it never
        // participates in a batch; its pages arrive through PublishRange.
        var batchCollections = model.Collections.Where(collection => collection.Window is null).ToArray();
        var modelCollections = batchCollections.Where(collection => collection.ElementKind == ViewModelValueKind.Model).ToArray();

        sb.AppendLine("    bool IRustVmBatchTarget.TryGetProperty(int propertyId, out RustVmBatchProperty property)");
        sb.AppendLine("    {");
        sb.AppendLine("        property = propertyId switch");
        sb.AppendLine("        {");
        foreach (var property in model.Properties)
        {
            // A nested model property is always clearable through SetNull.
            var nullable = (property.Nullable || property.Kind == ViewModelValueKind.Model).ToString().ToLowerInvariant();
            var isEnum = (property.Kind == ViewModelValueKind.Enum).ToString().ToLowerInvariant();
            sb.AppendLine(
                $"            {property.Id} => new RustVmBatchProperty(nameof({property.Name}), RustVmValueWireKind.{BatchWireKind(property.Kind)}, {nullable}, {isEnum}),");
        }
        sb.AppendLine("            _ => default,");
        sb.AppendLine("        };");
        sb.AppendLine("        return property.Name is not null;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmBatchTarget.TryGetCollection(int collectionId, out RustVmBatchCollectionInfo collection)");
        sb.AppendLine("    {");
        sb.AppendLine("        collection = collectionId switch");
        sb.AppendLine("        {");
        foreach (var collection in batchCollections)
        {
            sb.AppendLine(
                $"            {collection.Id} => new RustVmBatchCollectionInfo(nameof({collection.Name}), RustVmValueWireKind.{BatchWireKind(collection.ElementKind)}, {collection.Name}),");
        }
        sb.AppendLine("            _ => default,");
        sb.AppendLine("        };");
        sb.AppendLine("        return collection.Items is not null;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmBatchTarget.TryGetCommand(int commandId, out IRustVmBatchCommand command)");
        sb.AppendLine("    {");
        sb.AppendLine("        command = commandId switch");
        sb.AppendLine("        {");
        foreach (var command in model.Commands)
            sb.AppendLine($"            {command.Id} => {command.Name}Command,");
        sb.AppendLine("            _ => null!,");
        sb.AppendLine("        };");
        sb.AppendLine("        return command is not null;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmBatchTarget.IsEnumValueDefined(int propertyId, long value) => propertyId switch");
        sb.AppendLine("    {");
        foreach (var property in model.Properties.Where(property => property.Kind == ViewModelValueKind.Enum))
        {
            sb.AppendLine(
                $"        {property.Id} => global::System.Enum.IsDefined(typeof({CSharpEnumTypeName(ir, property.EnumName!)}), value),");
        }
        sb.AppendLine("        _ => false,");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    IDisposable IRustVmBatchTarget.CreateNestedProperty(int propertyId, IAvnRustViewModel model) => propertyId switch");
        sb.AppendLine("    {");
        foreach (var property in modelProperties)
        {
            sb.AppendLine(
                $"        {property.Id} => new {CSharpModelAdapterTypeName(ir, property.ModelName!)}(model, _dispatch, _post),");
        }
        sb.AppendLine("        _ => throw new ArgumentOutOfRangeException(nameof(propertyId)),");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    IDisposable IRustVmBatchTarget.CreateNestedElement(int collectionId, IAvnRustViewModel model) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in modelCollections)
        {
            sb.AppendLine(
                $"        {collection.Id} => new {CSharpModelAdapterTypeName(ir, collection.ElementModelName!)}(model, _dispatch, _post),");
        }
        sb.AppendLine("        _ => throw new ArgumentOutOfRangeException(nameof(collectionId)),");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmBatchTarget.CommitProperty(int propertyId, in RustVmBatchValue value, out IDisposable? replaced)");
        sb.AppendLine("    {");
        sb.AppendLine("        replaced = null;");
        sb.AppendLine("        switch (propertyId)");
        sb.AppendLine("        {");
        foreach (var property in model.Properties)
        {
            var field = $"_{Lower(property.Name)}";
            sb.AppendLine($"            case {property.Id}:");
            sb.AppendLine("            {");
            sb.AppendLine($"                var next = {CSharpBatchValue(ir, property)};");
            sb.AppendLine($"                if (Equals({field}, next)) return false;");
            if (property.Kind == ViewModelValueKind.Model)
                sb.AppendLine($"                replaced = {field};");
            sb.AppendLine($"                {field} = next;");
            sb.AppendLine("                return true;");
            sb.AppendLine("            }");
        }
        sb.AppendLine("            default: return false;");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmBatchTarget.CommitError(string propertyName, string? message) =>");
        sb.AppendLine("        RustVmBatchErrors.Set(_errors, propertyName, message);");
        sb.AppendLine();
        sb.AppendLine("    bool IRustVmTableSelectionBatchTarget.IsPostCollectionPropertyNotification(string propertyName, IReadOnlySet<string> changedCollections) => propertyName switch");
        sb.AppendLine("    {");
        foreach (var property in model.Properties)
        {
            var collections = model.Collections
                .Where(collection => collection.Table?.Selection is { } selection &&
                    (selection.SelectedIndexProperty == property.Name || selection.SelectedKeyProperty == property.Name))
                .Select(collection => $"changedCollections.Contains(nameof({collection.Name}))")
                .ToArray();
            if (collections.Length > 0)
                sb.AppendLine($"        nameof({property.Name}) => {string.Join(" || ", collections)},");
        }
        sb.AppendLine("        _ => false,");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    void IRustVmBatchTarget.RaisePropertyChanged(string propertyName) =>");
        sb.AppendLine("        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));");
        sb.AppendLine();
        sb.AppendLine("    void IRustVmBatchTarget.RaiseErrorsChanged(string propertyName) =>");
        sb.AppendLine("        ErrorsChanged?.Invoke(this, new DataErrorsChangedEventArgs(propertyName));");
        sb.AppendLine();
    }

    /// <summary>
    /// Emits the adapter's <c>IAvnRustVmSink5</c> implementation: appending,
    /// inserting and replacing elements of a scalar-number collection. Each
    /// entry point is a pure switch over a schema ID that dispatches onto the
    /// UI thread through the same <c>Apply</c> the v1/v2 sinks use, and an ID
    /// whose declared element kind does not match the call is rejected rather
    /// than coerced.
    /// </summary>
    private static void EmitCSharpNumberSinkMethods(
        StringBuilder sb,
        IReadOnlyList<ViewModelCollection> numberCollections)
    {
        foreach (var (suffix, transport, kind) in NumberElementKinds)
        {
            var matching = numberCollections.Where(collection => collection.ElementKind == kind).ToArray();
            sb.AppendLine($"    public int Add{suffix}(int collectionId, {transport} value) => collectionId switch");
            sb.AppendLine("    {");
            foreach (var collection in matching)
                sb.AppendLine($"        {collection.Id} => Apply(() => {collection.Name}.Add(value)),");
            sb.AppendLine("        _ => unchecked((int)0x80070057),");
            sb.AppendLine("    };");
            sb.AppendLine();

            sb.AppendLine($"    public int Insert{suffix}(int collectionId, int index, {transport} value) => collectionId switch");
            sb.AppendLine("    {");
            foreach (var collection in matching)
                sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index > (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.Insert(index, value); return 0; }}),");
            sb.AppendLine("        _ => unchecked((int)0x80070057),");
            sb.AppendLine("    };");
            sb.AppendLine();

            sb.AppendLine($"    public int Replace{suffix}(int collectionId, int index, {transport} value) => collectionId switch");
            sb.AppendLine("    {");
            foreach (var collection in matching)
                sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}[index] = value; return 0; }}),");
            sb.AppendLine("        _ => unchecked((int)0x80070057),");
            sb.AppendLine("    };");
            sb.AppendLine();
        }
    }

    private static readonly (string Suffix, string Transport, ViewModelValueKind Kind)[] NumberElementKinds =
    [
        ("Integer", "long", ViewModelValueKind.Integer),
        ("Double", "double", ViewModelValueKind.Double),
    ];

    /// <summary>
    /// Emits the adapter's <c>IAvnRustVmSink4</c> implementation: observable
    /// keyed maps, structured command results, async progress and windowed
    /// range publication. Every entry point is a pure switch over a schema ID
    /// that dispatches onto the UI thread through the same <c>Apply</c> used by
    /// the v1/v2 sinks, except <c>PublishRange</c>, which must not decode on the
    /// submitting stack and therefore goes through the range coordinator.
    /// </summary>
    private static void EmitCSharpShapeSinkMethods(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        IReadOnlyList<ViewModelMap> maps,
        IReadOnlyList<ViewModelCollection> windowedCollections,
        IReadOnlyList<ViewModelCommand> resultCommands,
        IReadOnlyList<ViewModelCommand> progressCommands,
        IReadOnlyList<ViewModelCommand> trackedCommands)
    {
        foreach (var (suffix, transport) in MapScalarKinds)
        {
            var matching = maps
                .Where(map => map.ValueKind != ViewModelValueKind.Model &&
                    TransportSuffix(WireKind(map.ValueKind)) == suffix)
                .ToArray();
            sb.AppendLine($"    public int MapSet{suffix}(int mapId, string? stringKey, long integerKey, {transport} value) => mapId switch");
            sb.AppendLine("    {");
            foreach (var map in matching)
                sb.AppendLine($"        {map.Id} => Apply(() => SetMapEntry({map.Name}, {CSharpMapKeyExpression(map)}, {CSharpMapScalarValue(map)}, nameof({map.Name}))),");
            sb.AppendLine("        _ => unchecked((int)0x80070057),");
            sb.AppendLine("    };");
            sb.AppendLine();
        }

        sb.AppendLine("    public int MapSetModel(int mapId, string? stringKey, long integerKey, IAvnRustViewModel? value) => mapId switch");
        sb.AppendLine("    {");
        foreach (var map in maps.Where(map => map.ValueKind == ViewModelValueKind.Model))
        {
            var adapter = CSharpModelAdapterTypeName(ir, map.ValueModelName!);
            sb.AppendLine($"        {map.Id} => value is null ? unchecked((int)0x80070057) : Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            var adapter = new {adapter}(value, _dispatch, _post);");
            sb.AppendLine($"            if ({map.Name}.Set({CSharpMapKeyExpression(map)}, adapter, out var displaced))");
            sb.AppendLine("            {");
            sb.AppendLine($"                RaiseMapChanged(nameof({map.Name}));");
            sb.AppendLine("                TryDispose(displaced);");
            sb.AppendLine("            }");
            sb.AppendLine("            else");
            sb.AppendLine("            {");
            sb.AppendLine("                TryDispose(adapter);");
            sb.AppendLine("            }");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int MapRemove(int mapId, string? stringKey, long integerKey) => mapId switch");
        sb.AppendLine("    {");
        foreach (var map in maps)
        {
            var dispose = map.ValueKind == ViewModelValueKind.Model ? " TryDispose(removed);" : "";
            sb.AppendLine($"        {map.Id} => Apply(() => {{ if ({map.Name}.Remove({CSharpMapKeyExpression(map)}, out var removed)) {{ RaiseMapChanged(nameof({map.Name}));{dispose} }} }}),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int MapClear(int mapId) => mapId switch");
        sb.AppendLine("    {");
        foreach (var map in maps)
        {
            var dispose = map.ValueKind == ViewModelValueKind.Model
                ? " foreach (var value in removed) TryDispose(value);"
                : "";
            sb.AppendLine($"        {map.Id} => Apply(() => {{ var removed = {map.Name}.Clear(); if (removed.Count > 0) RaiseMapChanged(nameof({map.Name}));{dispose} }}),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int SetCommandProgress(int commandId, int hasValue, double value, string? message) => commandId switch");
        sb.AppendLine("    {");
        foreach (var command in progressCommands)
        {
            sb.AppendLine($"        {command.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine("            var progress = hasValue != 0 ? RustAsyncCommands.ClampProgress(value) : (double?)null;");
            sb.AppendLine($"            SetField(ref _{Lower(command.Name)}Progress, progress, nameof({command.Name}Progress));");
            sb.AppendLine($"            SetField(ref _{Lower(command.Name)}ProgressMessage, message, nameof({command.Name}ProgressMessage));");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int SetCommandResult(int commandId, IAvnRustViewModel? result) => commandId switch");
        sb.AppendLine("    {");
        foreach (var command in resultCommands)
        {
            var adapter = CSharpModelAdapterTypeName(ir, command.ResultModelName!);
            sb.AppendLine($"        {command.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            var previous = _{Lower(command.Name)}Result;");
            sb.AppendLine($"            _{Lower(command.Name)}Result = result is null ? null : new {adapter}(result, _dispatch, _post);");
            sb.AppendLine($"            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof({command.Name}Result)));");
            sb.AppendLine("            TryDispose(previous);");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int SetCommandRunning(int commandId, int running) => commandId switch");
        sb.AppendLine("    {");
        foreach (var command in trackedCommands)
        {
            sb.AppendLine($"        {command.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine("            var isRunning = running != 0;");
            sb.AppendLine($"            SetField(ref _{Lower(command.Name)}IsRunning, isRunning, nameof({command.Name}IsRunning));");
            if (command.SupportsCancellation)
            {
                sb.AppendLine($"            if (!isRunning) _{Lower(command.Name)}Operation = 0;");
                sb.AppendLine($"            Cancel{command.Name}Command.SetEnabled(isRunning);");
            }
            if (command.SupportsProgress)
            {
                sb.AppendLine("            if (!isRunning)");
                sb.AppendLine("            {");
                sb.AppendLine($"                SetField(ref _{Lower(command.Name)}Progress, null, nameof({command.Name}Progress));");
                sb.AppendLine($"                SetField(ref _{Lower(command.Name)}ProgressMessage, null, nameof({command.Name}ProgressMessage));");
                sb.AppendLine("            }");
            }
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    /// <summary>");
        sb.AppendLine("    /// Enqueues one range batch. Like <see cref=\"SubmitBatch\"/> this never reads,");
        sb.AppendLine("    /// applies or completes the batch on the submitting (Rust worker) stack.");
        sb.AppendLine("    /// </summary>");
        if (windowedCollections.Count > 0)
            sb.AppendLine("    public int PublishRange(IAvnRustVmRangeBatch? batch) => _ranges.Publish(batch);");
        else
            sb.AppendLine("    public int PublishRange(IAvnRustVmRangeBatch? batch) => unchecked((int)0x80070057);");
        sb.AppendLine();

        if (maps.Count > 0)
        {
            sb.AppendLine("    private void RaiseMapChanged(string name) =>");
            sb.AppendLine("        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));");
            sb.AppendLine();
            sb.AppendLine("    private void SetMapEntry<TKey, TValue>(RustObservableMap<TKey, TValue> map, TKey key, TValue value, string name)");
            sb.AppendLine("        where TKey : notnull");
            sb.AppendLine("    {");
            sb.AppendLine("        if (map.Set(key, value, out _)) RaiseMapChanged(name);");
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        _ = model;
    }

    private static readonly (string Suffix, string Transport)[] MapScalarKinds =
    [
        ("String", "string?"),
        ("Integer", "long"),
        ("Boolean", "int"),
        ("Double", "double"),
    ];

    private static string CSharpMapKeyType(ViewModelMap map) =>
        map.KeyKind == ViewModelValueKind.String ? "string" : "long";

    private static string CSharpMapKeyExpression(ViewModelMap map) =>
        map.KeyKind == ViewModelValueKind.String ? "stringKey ?? \"\"" : "integerKey";

    private static string CSharpMapScalarValue(ViewModelMap map) => map.ValueKind switch
    {
        ViewModelValueKind.String => "value ?? \"\"",
        ViewModelValueKind.Boolean => "value != 0",
        _ => "value",
    };

    private static string CSharpMapValueType(ViewModelIr ir, ViewModelMap map) => map.ValueKind switch
    {
        ViewModelValueKind.String => "string",
        ViewModelValueKind.Integer => "long",
        ViewModelValueKind.Boolean => "bool",
        ViewModelValueKind.Double => "double",
        ViewModelValueKind.Model => CSharpModelAdapterTypeName(ir, map.ValueModelName!),
        _ => throw new ArgumentOutOfRangeException(nameof(map)),
    };

    private static string BatchWireKind(ViewModelValueKind kind) => kind switch
    {
        ViewModelValueKind.String => "String",
        ViewModelValueKind.Integer or ViewModelValueKind.Enum => "Integer",
        ViewModelValueKind.Boolean => "Boolean",
        ViewModelValueKind.Double => "Double",
        ViewModelValueKind.Model => "Model",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    /// <summary>
    /// Builds the expression that turns one already validated and staged
    /// <c>RustVmBatchValue</c> into this property's strongly typed field value.
    /// </summary>
    private static string CSharpBatchValue(ViewModelIr ir, ViewModelProperty property) => property.Kind switch
    {
        // A nullable string reads null straight from the decoded value: only
        // SetString carries text, so SetNull naturally lands on null.
        ViewModelValueKind.String => property.Nullable ? "value.Text" : "value.Text ?? \"\"",
        ViewModelValueKind.Integer => "value.Integer",
        ViewModelValueKind.Boolean => "value.Boolean",
        ViewModelValueKind.Double => "value.Double",
        ViewModelValueKind.Enum => $"({CSharpEnumTypeName(ir, property.EnumName!)})value.Integer",
        ViewModelValueKind.Model => $"({CSharpModelAdapterTypeName(ir, property.ModelName!)}?)value.Model",
        _ => throw new ArgumentOutOfRangeException(nameof(property)),
    };

    private static void EmitCSharpSinkMethods(StringBuilder sb, ViewModelIr ir, ViewModelDefinition model)
    {
        foreach (var wireKind in ScalarWireKinds)
        {
            var properties = model.Properties.Where(property => WireKind(property.Kind) == wireKind).ToArray();
            sb.AppendLine($"    public int Set{TransportSuffix(wireKind)}(int propertyId, {CSharpTransportType(wireKind)} value)");
            sb.AppendLine("    {");
            sb.AppendLine("        var inbound = _inboundWrites.MarkPublication(propertyId);");
            sb.AppendLine("        return propertyId switch");
            sb.AppendLine("        {");
            foreach (var property in properties)
            {
                if (property.Kind == ViewModelValueKind.Enum)
                {
                    var enumType = CSharpEnumTypeName(ir, property.EnumName!);
                    var converted = CSharpFromTransport(ir, property, "value");
                    sb.AppendLine(
                        $"            {property.Id} => !global::System.Enum.IsDefined(typeof({enumType}), value) ? unchecked((int)0x80070057) : Apply(() => {{ var converted = {converted}; _inboundWrites.CommitPublication(propertyId, inbound); if (!Equals(_{Lower(property.Name)}, converted)) SetField(ref _{Lower(property.Name)}, converted, nameof({property.Name})); }}),");
                }
                else
                {
                    var converted = CSharpFromTransport(ir, property, "value");
                    sb.AppendLine($"            {property.Id} => Apply(() => {{ var converted = {converted}; _inboundWrites.CommitPublication(propertyId, inbound); if (!Equals(_{Lower(property.Name)}, converted)) SetField(ref _{Lower(property.Name)}, converted, nameof({property.Name})); }}),");
                }
            }
            sb.AppendLine("            _ => unchecked((int)0x80070057),");
            sb.AppendLine("        };");
            sb.AppendLine("    }");
            sb.AppendLine();
        }

        var nullableProperties = model.Properties.Where(property => property.Nullable && property.Kind != ViewModelValueKind.Model).ToArray();
        sb.AppendLine("    public int SetNull(int propertyId)");
        sb.AppendLine("    {");
        sb.AppendLine("        var inbound = _inboundWrites.MarkPublication(propertyId);");
        sb.AppendLine("        return propertyId switch");
        sb.AppendLine("        {");
        foreach (var property in nullableProperties)
            sb.AppendLine($"            {property.Id} => Apply(() => {{ if (_{Lower(property.Name)} is not null) {{ _inboundWrites.CommitPublication(propertyId, inbound); SetField(ref _{Lower(property.Name)}, null, nameof({property.Name})); }} }}),");
        sb.AppendLine("            _ => unchecked((int)0x80070057),");
        sb.AppendLine("        };");
        sb.AppendLine("    }");
        sb.AppendLine();

        var modelProperties = model.Properties.Where(property => property.Kind == ViewModelValueKind.Model).ToArray();
        sb.AppendLine("    public int SetModel(int propertyId, IAvnRustViewModel? model) => propertyId switch");
        sb.AppendLine("    {");
        foreach (var property in modelProperties)
        {
            var adapterType = CSharpModelAdapterTypeName(ir, property.ModelName!);
            sb.AppendLine($"        {property.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            var previous = _{Lower(property.Name)};");
            sb.AppendLine($"            _{Lower(property.Name)} = model is null ? null : new {adapterType}(model, _dispatch, _post);");
            sb.AppendLine($"            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof({property.Name})));");
            sb.AppendLine("            previous?.Dispose();");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        // A windowed collection is deliberately excluded from every
        // materializing path: it has no managed element list to insert into,
        // and its pages arrive through IAvnRustVmSink4.PublishRange instead.
        var materialized = model.Collections.Where(collection => collection.Window is null).ToArray();
        var modelCollections = materialized.Where(collection => collection.ElementKind == ViewModelValueKind.Model).ToArray();
        var stringCollections = materialized.Where(collection => collection.ElementKind == ViewModelValueKind.String).ToArray();
        // Removal, movement and clearing carry no element value, so a
        // scalar-number collection rides the same already-published v2 calls
        // as a string one instead of duplicating them on the v5 sink.
        var numberCollections = materialized.Where(IsNumberCollection).ToArray();

        sb.AppendLine("    public int AddModel(int collectionId, IAvnRustViewModel? model) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in modelCollections)
        {
            var adapterType = CSharpModelAdapterTypeName(ir, collection.ElementModelName!);
            sb.AppendLine($"        {collection.Id} => model is null ? unchecked((int)0x80070057) : Apply(() => {collection.Name}.Add(new {adapterType}(model, _dispatch, _post))),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int InsertString(int collectionId, int index, string? value) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in stringCollections)
            sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index > (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.Insert(index, value ?? \"\"); return 0; }}),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int InsertModel(int collectionId, int index, IAvnRustViewModel? model) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in modelCollections)
        {
            var adapterType = CSharpModelAdapterTypeName(ir, collection.ElementModelName!);
            sb.AppendLine($"        {collection.Id} => model is null ? unchecked((int)0x80070057) : Apply(() => {{ if ((uint)index > (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.Insert(index, new {adapterType}(model, _dispatch, _post)); return 0; }}),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int ReplaceString(int collectionId, int index, string? value) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in stringCollections)
            sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}[index] = value ?? \"\"; return 0; }}),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int ReplaceModel(int collectionId, int index, IAvnRustViewModel? model) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in modelCollections)
        {
            var adapterType = CSharpModelAdapterTypeName(ir, collection.ElementModelName!);
            sb.AppendLine($"        {collection.Id} => model is null ? unchecked((int)0x80070057) : Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057);");
            sb.AppendLine($"            var previous = {collection.Name}[index];");
            sb.AppendLine($"            {collection.Name}[index] = new {adapterType}(model, _dispatch, _post);");
            sb.AppendLine("            previous.Dispose();");
            sb.AppendLine("            return 0;");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int RemoveAt(int collectionId, int index) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in stringCollections)
            sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.RemoveAt(index); return 0; }}),");
        foreach (var collection in numberCollections)
            sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.RemoveAt(index); return 0; }}),");
        foreach (var collection in modelCollections)
        {
            sb.AppendLine($"        {collection.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            if ((uint)index >= (uint){collection.Name}.Count) return unchecked((int)0x80070057);");
            sb.AppendLine($"            var item = {collection.Name}[index];");
            sb.AppendLine($"            {collection.Name}.RemoveAt(index);");
            sb.AppendLine("            item.Dispose();");
            sb.AppendLine("            return 0;");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int MoveItem(int collectionId, int fromIndex, int toIndex) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in materialized)
            sb.AppendLine($"        {collection.Id} => Apply(() => {{ if ((uint)fromIndex >= (uint){collection.Name}.Count || (uint)toIndex >= (uint){collection.Name}.Count) return unchecked((int)0x80070057); {collection.Name}.Move(fromIndex, toIndex); return 0; }}),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int ClearCollection(int collectionId) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in stringCollections)
            sb.AppendLine($"        {collection.Id} => Apply({collection.Name}.Clear),");
        foreach (var collection in numberCollections)
            sb.AppendLine($"        {collection.Id} => Apply({collection.Name}.Clear),");
        foreach (var collection in modelCollections)
        {
            sb.AppendLine($"        {collection.Id} => Apply(() =>");
            sb.AppendLine("        {");
            sb.AppendLine($"            foreach (var item in {collection.Name}) item.Dispose();");
            sb.AppendLine($"            {collection.Name}.Clear();");
            sb.AppendLine("        }),");
        }
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int SetCommandEnabled(int commandId, int enabled) => commandId switch");
        sb.AppendLine("    {");
        foreach (var command in model.Commands)
            sb.AppendLine($"        {command.Id} => Apply(() => {command.Name}Command.SetEnabled(enabled != 0)),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int SetPropertyError(int propertyId, string? message) => propertyId switch");
        sb.AppendLine("    {");
        foreach (var property in model.Properties)
            sb.AppendLine($"        {property.Id} => Apply(() => SetError(nameof({property.Name}), message)),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();

        sb.AppendLine("    public int AddString(int collectionId, string? value) => collectionId switch");
        sb.AppendLine("    {");
        foreach (var collection in stringCollections)
            sb.AppendLine($"        {collection.Id} => Apply(() => {collection.Name}.Add(value ?? \"\")),");
        sb.AppendLine("        _ => unchecked((int)0x80070057),");
        sb.AppendLine("    };");
        sb.AppendLine();
    }

    /// <summary>
    /// Emits the declared display projection as a <c>ToString()</c> override.
    /// </summary>
    /// <remarks>
    /// Presentation and UI automation fall back to <c>ToString()</c> for a data
    /// item they cannot template, so without this a table row, list item or
    /// tree node reads as the generated adapter's CLR type name - which is what
    /// a screen reader and an automation client would then report.
    /// </remarks>
    private static void EmitCSharpDisplayPath(StringBuilder sb, ViewModelIr ir, ViewModelDefinition model)
    {
        if (model.DisplayPath is not { } path)
            return;
        var segments = path.Split('.');
        var current = model;
        var expression = new StringBuilder();
        var nullable = false;
        for (var index = 0; index < segments.Length; index++)
        {
            var property = current.Properties.Single(candidate => candidate.Name == segments[index]);
            expression.Append(index == 0 ? property.Name : (nullable ? "?." : ".") + property.Name);
            if (index == segments.Length - 1)
            {
                nullable |= property.Nullable;
                break;
            }

            nullable = true;
            current = ir.Models.Single(candidate => candidate.Name == property.ModelName);
        }

        sb.AppendLine();
        sb.AppendLine("    /// <summary>");
        sb.AppendLine($"    /// The declared display projection (<c>{path}</c>). Presentation and UI");
        sb.AppendLine("    /// automation fall back to this for an untemplated data item, so it reports");
        sb.AppendLine("    /// the row's own text rather than this adapter's CLR type name.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public override string ToString() => {expression}{(nullable ? " ?? \"\"" : "")};");
    }

    /// <summary>
    /// Emits the generated menu surface for one model: application menus,
    /// accelerator-only menus and context menus.
    /// </summary>
    /// <remarks>
    /// Menus are presentation, so none of this crosses the ABI. Every item is
    /// wired to an already generated command or writable property on the typed
    /// adapter, with no reflection anywhere, which keeps the whole path
    /// NativeAOT-safe.
    /// </remarks>
    private static string EmitCSharpMenus(ViewModelIr ir, ViewModelDefinition model)
    {
        var adapter = $"{model.Name}Adapter";
        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine("using System;");
        sb.AppendLine("using System.Collections.Generic;");
        sb.AppendLine("using Avalonia.Controls;");
        sb.AppendLine("using Avalonia.Input;");
        sb.AppendLine("using Avalonia.Rust;");
        sb.AppendLine();
        sb.AppendLine($"namespace {model.ManagedNamespace};");
        sb.AppendLine();
        sb.AppendLine("/// <summary>");
        sb.AppendLine($"/// Generated menus, keyboard accelerators and recent-file projection for <c>{model.Name}</c>.");
        sb.AppendLine("/// </summary>");
        sb.AppendLine($"public static class {model.Name}Menus");
        sb.AppendLine("{");
        if (model.RecentFiles is { } recent)
        {
            sb.AppendLine("    /// <summary>Maximum recent-file entries projected into a submenu.</summary>");
            sb.AppendLine($"    public const int RecentFilesCapacity = {recent.Capacity};");
            sb.AppendLine();
            sb.AppendLine("    /// <summary>Placeholder header shown while the recent-file list is empty.</summary>");
            sb.AppendLine("    public const string RecentFilesEmptyHeader = \"(no recent files)\";");
            sb.AppendLine();
        }

        foreach (var menu in model.Menus)
        {
            switch (menu.Kind)
            {
                case ViewModelMenuKind.Application:
                    EmitApplicationMenu(sb, ir, model, menu, adapter);
                    break;
                case ViewModelMenuKind.Accelerators:
                    EmitAcceleratorMenu(sb, ir, model, menu, adapter);
                    break;
                case ViewModelMenuKind.Context:
                    EmitContextMenuFactory(sb, ir, model, menu, adapter);
                    break;
            }
        }

        sb.AppendLine("}");
        foreach (var menu in model.Menus.Where(menu => menu.Kind == ViewModelMenuKind.Context))
        {
            sb.AppendLine();
            sb.AppendLine("/// <summary>");
            sb.AppendLine($"/// Generated context menu <c>{menu.Name}</c> for <c>{model.Name}</c>.");
            sb.AppendLine("/// </summary>");
            sb.AppendLine("/// <remarks>");
            sb.AppendLine("/// Declared directly in compiled AXAML - for example");
            sb.AppendLine($"/// <c>&lt;TableView.ContextMenu&gt;&lt;vm:{model.Name}{menu.Name}ContextMenu /&gt;&lt;/TableView.ContextMenu&gt;</c>.");
            sb.AppendLine("/// A context menu is parented to the control it is attached to, so it inherits");
            sb.AppendLine("/// that control's data context and binds itself when the data context arrives.");
            sb.AppendLine("/// </remarks>");
            sb.AppendLine($"public sealed class {model.Name}{menu.Name}ContextMenu : ContextMenu");
            sb.AppendLine("{");
            sb.AppendLine("    private RustMenuScope? _scope;");
            sb.AppendLine();
            sb.AppendLine("    protected override void OnDataContextChanged(EventArgs e)");
            sb.AppendLine("    {");
            sb.AppendLine("        base.OnDataContextChanged(e);");
            sb.AppendLine("        _scope?.Dispose();");
            sb.AppendLine("        _scope = null;");
            sb.AppendLine($"        if (DataContext is {adapter} model)");
            sb.AppendLine("        {");
            sb.AppendLine("            var scope = new RustMenuScope(model);");
            sb.AppendLine("            _scope = scope;");
            sb.AppendLine($"            ItemsSource = {model.Name}Menus.Create{menu.Name}Items(model, scope);");
            sb.AppendLine("        }");
            sb.AppendLine("        else");
            sb.AppendLine("        {");
            sb.AppendLine("            ItemsSource = null;");
            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");
        }

        return sb.ToString();
    }

    private static void EmitApplicationMenu(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        ViewModelMenu menu,
        string adapter)
    {
        sb.AppendLine("    /// <summary>");
        sb.AppendLine($"    /// Builds the <c>{menu.Name}</c> application menu, appending every declared");
        sb.AppendLine("    /// accelerator to <paramref name=\"keyBindings\"/>.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public static NativeMenu Create{menu.Name}({adapter} model, RustMenuScope scope, IList<KeyBinding>? keyBindings = null)");
        sb.AppendLine("    {");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(model);");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(scope);");
        sb.AppendLine("        var menu = new NativeMenu();");
        EmitNativeMenuItems(sb, ir, model, menu.Items, "menu.Items", "        ", keyBindings: true);
        sb.AppendLine("        return menu;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    /// <summary>");
        sb.AppendLine($"    /// Attaches the <c>{menu.Name}</c> menu and its accelerators to a top-level.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine("    /// <remarks>");
        sb.AppendLine("    /// The menu is exported natively where the platform has a menu bar (macOS)");
        sb.AppendLine("    /// and rendered in-window by a <c>NativeMenuBar</c> control everywhere else;");
        sb.AppendLine("    /// both read the same attached value, so there is no platform branch. Only a");
        sb.AppendLine("    /// real native menu bar handles its own shortcuts, so the declared gestures");
        sb.AppendLine("    /// are additionally installed as key bindings on the top-level.");
        sb.AppendLine("    /// </remarks>");
        sb.AppendLine($"    public static RustMenuAttachment Attach{menu.Name}(TopLevel target, {adapter} model)");
        sb.AppendLine("    {");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(target);");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(model);");
        sb.AppendLine("        var scope = new RustMenuScope(model);");
        sb.AppendLine("        try");
        sb.AppendLine("        {");
        sb.AppendLine("            var keyBindings = new List<KeyBinding>();");
        sb.AppendLine($"            var menu = Create{menu.Name}(model, scope, keyBindings);");
        sb.AppendLine("            return RustMenuAttachment.Attach(target, menu, keyBindings, scope);");
        sb.AppendLine("        }");
        sb.AppendLine("        catch");
        sb.AppendLine("        {");
        sb.AppendLine("            scope.Dispose();");
        sb.AppendLine("            throw;");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine();
    }

    private static void EmitAcceleratorMenu(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        ViewModelMenu menu,
        string adapter)
    {
        sb.AppendLine("    /// <summary>");
        sb.AppendLine($"    /// Builds the standalone <c>{menu.Name}</c> accelerators, which appear on no menu.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public static IReadOnlyList<KeyBinding> Create{menu.Name}({adapter} model, RustMenuScope scope)");
        sb.AppendLine("    {");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(model);");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(scope);");
        sb.AppendLine("        var keyBindings = new List<KeyBinding>();");
        EmitNativeMenuItems(sb, ir, model, menu.Items, null, "        ", keyBindings: true);
        sb.AppendLine("        return keyBindings;");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine($"    /// <summary>Installs the <c>{menu.Name}</c> accelerators on an input element.</summary>");
        sb.AppendLine($"    public static IDisposable Attach{menu.Name}(InputElement target, {adapter} model)");
        sb.AppendLine("    {");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(target);");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(model);");
        sb.AppendLine("        var scope = new RustMenuScope(model);");
        sb.AppendLine("        try");
        sb.AppendLine("        {");
        sb.AppendLine($"            RustMenu.AttachKeyBindings(target, Create{menu.Name}(model, scope), scope);");
        sb.AppendLine("            return scope;");
        sb.AppendLine("        }");
        sb.AppendLine("        catch");
        sb.AppendLine("        {");
        sb.AppendLine("            scope.Dispose();");
        sb.AppendLine("            throw;");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine();
    }

    private static void EmitContextMenuFactory(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        ViewModelMenu menu,
        string adapter)
    {
        sb.AppendLine("    /// <summary>");
        sb.AppendLine($"    /// Builds the items of the <c>{menu.Name}</c> context menu.");
        sb.AppendLine("    /// </summary>");
        sb.AppendLine($"    public static IReadOnlyList<Control> Create{menu.Name}Items({adapter} model, RustMenuScope scope)");
        sb.AppendLine("    {");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(model);");
        sb.AppendLine("        ArgumentNullException.ThrowIfNull(scope);");
        sb.AppendLine("        var items = new List<Control>();");
        EmitControlMenuItems(sb, ir, model, menu.Items, "items", "        ");
        sb.AppendLine("        return items;");
        sb.AppendLine("    }");
        sb.AppendLine();
    }

    /// <summary>
    /// Emits <see cref="NativeMenuItem"/> construction for one item list.
    /// <paramref name="parent"/> is null for an accelerator-only menu, which
    /// produces key bindings and no visual items at all.
    /// </summary>
    private static void EmitNativeMenuItems(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        IReadOnlyList<ViewModelMenuItem> items,
        string? parent,
        string indent,
        bool keyBindings)
    {
        foreach (var item in items)
        {
            var name = Lower(item.Name);
            switch (item.Kind)
            {
                case ViewModelMenuItemKind.Separator:
                    if (parent is not null)
                        sb.AppendLine($"{indent}{parent}.Add(new NativeMenuItemSeparator());");
                    break;
                case ViewModelMenuItemKind.Submenu:
                    sb.AppendLine($"{indent}var {name}Item = new NativeMenuItem({CSharpString(item.Header)});");
                    sb.AppendLine($"{indent}var {name}Menu = new NativeMenu();");
                    sb.AppendLine($"{indent}{name}Item.Menu = {name}Menu;");
                    EmitSubmenuEnabled(sb, item, $"{name}Item", indent);
                    EmitNativeMenuItems(sb, ir, model, item.Items, $"{name}Menu.Items", indent, keyBindings);
                    if (parent is not null)
                        sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    break;
                case ViewModelMenuItemKind.RecentFiles:
                    var recent = model.RecentFiles!;
                    sb.AppendLine($"{indent}var {name}Item = new NativeMenuItem({CSharpString(item.Header)});");
                    sb.AppendLine($"{indent}var {name}Menu = new NativeMenu();");
                    sb.AppendLine($"{indent}{name}Item.Menu = {name}Menu;");
                    sb.AppendLine($"{indent}var {name}Activate = new RustMenuCommand(parameter => model.{recent.ActivateCommand}Command.Execute(parameter));");
                    sb.AppendLine($"{indent}{name}Activate.TrackSource(model.{recent.ActivateCommand}Command, scope);");
                    sb.AppendLine($"{indent}scope.ObserveCollection(model.{recent.Collection}, () => RustMenu.FillRecentFiles({name}Menu.Items, model.{recent.Collection}, {name}Activate, RecentFilesEmptyHeader, RecentFilesCapacity));");
                    if (parent is not null)
                        sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    break;
                default:
                    EmitMenuCommand(sb, ir, model, item, name, indent);
                    if (parent is not null)
                    {
                        sb.AppendLine($"{indent}var {name}Item = new NativeMenuItem({CSharpString(item.Header)}) {{ Command = {name}Command }};");
                        if (item.Kind == ViewModelMenuItemKind.Toggle)
                            sb.AppendLine($"{indent}{name}Item.ToggleType = MenuItemToggleType.CheckBox;");
                        if (item.Kind == ViewModelMenuItemKind.Radio)
                            sb.AppendLine($"{indent}{name}Item.ToggleType = MenuItemToggleType.Radio;");
                        if (item.Gesture is { } gesture)
                            sb.AppendLine($"{indent}{name}Item.Gesture = RustMenu.ParseGesture({CSharpString(gesture)});");
                        EmitCheckedObserver(sb, ir, model, item, $"{name}Item", indent);
                        sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    }

                    if (keyBindings && item.Gesture is { } accelerator)
                    {
                        var target = parent is null ? "keyBindings" : "keyBindings?";
                        sb.AppendLine(
                            $"{indent}{target}.Add(new KeyBinding {{ Gesture = RustMenu.ParseGesture({CSharpString(accelerator)})!, Command = {name}Command }});");
                    }

                    break;
            }
        }
    }

    private static void EmitControlMenuItems(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        IReadOnlyList<ViewModelMenuItem> items,
        string parent,
        string indent)
    {
        foreach (var item in items)
        {
            var name = Lower(item.Name);
            switch (item.Kind)
            {
                case ViewModelMenuItemKind.Separator:
                    sb.AppendLine($"{indent}{parent}.Add(new Separator());");
                    break;
                case ViewModelMenuItemKind.Submenu:
                    sb.AppendLine($"{indent}var {name}Item = new MenuItem {{ Header = {CSharpString(item.Header)} }};");
                    sb.AppendLine($"{indent}var {name}Items = new List<Control>();");
                    EmitControlMenuItems(sb, ir, model, item.Items, $"{name}Items", indent);
                    sb.AppendLine($"{indent}{name}Item.ItemsSource = {name}Items;");
                    EmitSubmenuEnabled(sb, item, $"{name}Item", indent);
                    sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    break;
                case ViewModelMenuItemKind.RecentFiles:
                    var recent = model.RecentFiles!;
                    sb.AppendLine($"{indent}var {name}Item = new MenuItem {{ Header = {CSharpString(item.Header)} }};");
                    sb.AppendLine($"{indent}var {name}Activate = new RustMenuCommand(parameter => model.{recent.ActivateCommand}Command.Execute(parameter));");
                    sb.AppendLine($"{indent}{name}Activate.TrackSource(model.{recent.ActivateCommand}Command, scope);");
                    sb.AppendLine($"{indent}scope.ObserveCollection(model.{recent.Collection}, () => RustMenu.FillRecentFiles({name}Item, model.{recent.Collection}, {name}Activate, RecentFilesEmptyHeader, RecentFilesCapacity));");
                    sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    break;
                default:
                    EmitMenuCommand(sb, ir, model, item, name, indent);
                    sb.AppendLine($"{indent}var {name}Item = new MenuItem {{ Header = {CSharpString(item.Header)}, Command = {name}Command }};");
                    if (item.Kind == ViewModelMenuItemKind.Toggle)
                        sb.AppendLine($"{indent}{name}Item.ToggleType = MenuItemToggleType.CheckBox;");
                    if (item.Kind == ViewModelMenuItemKind.Radio)
                    {
                        sb.AppendLine($"{indent}{name}Item.ToggleType = MenuItemToggleType.Radio;");
                        sb.AppendLine($"{indent}{name}Item.GroupName = {CSharpString(item.RadioProperty)};");
                    }

                    if (item.Gesture is { } gesture)
                        sb.AppendLine($"{indent}{name}Item.InputGesture = RustMenu.ParseGesture({CSharpString(gesture)});");
                    EmitCheckedObserver(sb, ir, model, item, $"{name}Item", indent);
                    sb.AppendLine($"{indent}{parent}.Add({name}Item);");
                    break;
            }
        }
    }

    /// <summary>
    /// Emits the single <see cref="System.Windows.Input.ICommand"/> behind one
    /// item. A toggle or radio item writes its declared property first and then
    /// invokes its optional command, so Rust observes the new state.
    /// </summary>
    private static void EmitMenuCommand(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        ViewModelMenuItem item,
        string name,
        string indent)
    {
        var body = new List<string>();
        if (item.Kind == ViewModelMenuItemKind.Toggle)
            body.Add($"model.{item.ToggleProperty} = !model.{item.ToggleProperty};");
        if (item.Kind == ViewModelMenuItemKind.Radio)
            body.Add($"model.{item.RadioProperty} = {RadioValueExpression(ir, model, item)};");
        if (item.Command is { } command)
        {
            body.Add(item.Kind == ViewModelMenuItemKind.Command
                ? $"model.{command}Command.Execute(parameter);"
                : $"model.{command}Command.Execute(null);");
        }

        var conditions = new List<string>();
        if (item.Command is { } gate)
            conditions.Add($"model.{gate}Command.CanExecute(null)");
        if (item.IsEnabledProperty is { } enabled)
            conditions.Add($"model.{enabled}");
        var canExecute = conditions.Count == 0 ? "null" : $"() => {string.Join(" && ", conditions)}";
        sb.AppendLine($"{indent}var {name}Command = new RustMenuCommand(parameter => {{ {string.Join(" ", body)} }}, {canExecute});");
        if (item.Command is { } tracked)
            sb.AppendLine($"{indent}{name}Command.TrackSource(model.{tracked}Command, scope);");
        if (item.IsEnabledProperty is { } observed)
            sb.AppendLine($"{indent}scope.Observe({CSharpString(observed)}, {name}Command.RaiseCanExecuteChanged);");
    }

    /// <summary>
    /// A submenu carries no command, so assigning <c>IsEnabled</c> directly is
    /// safe there: nothing recomputes it from a command's <c>CanExecute</c>.
    /// </summary>
    private static void EmitSubmenuEnabled(
        StringBuilder sb,
        ViewModelMenuItem item,
        string name,
        string indent)
    {
        if (item.IsEnabledProperty is not { } enabled)
            return;
        sb.AppendLine($"{indent}scope.Observe({CSharpString(enabled)}, () => {name}.IsEnabled = model.{enabled});");
    }

    private static void EmitCheckedObserver(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        ViewModelMenuItem item,
        string name,
        string indent)
    {
        switch (item.Kind)
        {
            case ViewModelMenuItemKind.Toggle:
                sb.AppendLine($"{indent}scope.Observe({CSharpString(item.ToggleProperty)}, () => {name}.IsChecked = model.{item.ToggleProperty});");
                break;
            case ViewModelMenuItemKind.Radio:
                sb.AppendLine($"{indent}scope.Observe({CSharpString(item.RadioProperty)}, () => {name}.IsChecked = {RadioMatchExpression(ir, model, item)});");
                break;
        }
    }

    private static string RadioValueExpression(ViewModelIr ir, ViewModelDefinition model, ViewModelMenuItem item)
    {
        var property = model.Properties.Single(candidate => candidate.Name == item.RadioProperty);
        return property.Kind == ViewModelValueKind.Enum
            ? $"{CSharpEnumTypeName(ir, property.EnumName!)}.{item.RadioValue}"
            : CSharpString(item.RadioValue);
    }

    private static string RadioMatchExpression(ViewModelIr ir, ViewModelDefinition model, ViewModelMenuItem item)
    {
        var property = model.Properties.Single(candidate => candidate.Name == item.RadioProperty);
        return property.Kind == ViewModelValueKind.Enum
            ? $"model.{property.Name} == {RadioValueExpression(ir, model, item)}"
            : $"string.Equals(model.{property.Name}, {CSharpString(item.RadioValue)}, StringComparison.Ordinal)";
    }

    private static string EmitCSharpMetadata(ViewModelIr ir, ViewModelDefinition model)
    {        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine("using System.Collections.Generic;");
        sb.AppendLine("using Avalonia.Rust;");
        if (model.Collections.Any(collection => collection.Table is not null))
        {
            sb.AppendLine("using Avalonia.Controls;");
            sb.AppendLine("using Avalonia.Layout;");
        }
        sb.AppendLine();
        sb.AppendLine($"namespace {model.ManagedNamespace};");
        sb.AppendLine();
        sb.AppendLine($"public static class {model.Name}Metadata");
        sb.AppendLine("{");
        sb.AppendLine($"    public static RustViewModelDescriptor Descriptor {{ get; }} = new(");
        sb.AppendLine($"        {model.Id},");
        sb.AppendLine($"        {JsonSerializer.Serialize(model.Name)},");
        sb.AppendLine("        [");
        foreach (var property in model.Properties)
        {
            var nestedDescriptor = property.Kind == ViewModelValueKind.Model
                ? $"{CSharpModelMetadataTypeName(ir, property.ModelName!)}.Descriptor"
                : "null";
            sb.AppendLine(
                $"            new({property.Id}, {JsonSerializer.Serialize(property.Name)}, RustViewModelValueKind.{property.Kind}, {property.Writable.ToString().ToLowerInvariant()}, {property.Nullable.ToString().ToLowerInvariant()}, {CSharpInitial(ir, property)}, {nestedDescriptor}),");
        }
        sb.AppendLine("        ],");
        sb.AppendLine("        [");
        foreach (var collection in model.Collections)
        {
            // A recursive (tree children) collection deliberately publishes a
            // null element descriptor: its element model is its own owner, and
            // a self-referencing static initializer would be a cycle. The
            // Recursive flag plus the tree descriptor carry that shape instead.
            var elementDescriptor = collection.ElementKind == ViewModelValueKind.Model && !collection.Recursive
                ? $"{CSharpModelMetadataTypeName(ir, collection.ElementModelName!)}.Descriptor"
                : "null";
            var table = collection.Table is null ? "null" : $"Create{collection.Name}Table()";
            var window = collection.Window is null
                ? "null"
                : $"new({collection.Window.PageSize}, {collection.Window.MaxLivePages})";
            var tree = collection.Tree is null
                ? "null"
                : $"new({JsonSerializer.Serialize(collection.Tree.ChildrenCollection)}, {JsonSerializer.Serialize(collection.Tree.HeaderPath)}, {CSharpNullableString(collection.Tree.HasChildrenProperty)})";
            sb.AppendLine($"            new({collection.Id}, {JsonSerializer.Serialize(collection.Name)}, RustViewModelValueKind.{collection.ElementKind}, {elementDescriptor}, {table}, {window}, {tree}, {collection.Recursive.ToString().ToLowerInvariant()}),");
        }
        sb.AppendLine("        ],");
        sb.AppendLine("        [");
        foreach (var command in model.Commands)
        {
            var parameter = command.ParameterProperty is null
                ? "null"
                : JsonSerializer.Serialize(command.ParameterProperty);
            var result = command.ResultModelName is null
                ? "null"
                : $"{CSharpModelMetadataTypeName(ir, command.ResultModelName)}.Descriptor";
            sb.AppendLine(
                $"            new({command.Id}, {JsonSerializer.Serialize(command.Name + "Command")}, {command.IsAsync.ToString().ToLowerInvariant()}, {parameter}, {AcceptsCommandParameter(model, command).ToString().ToLowerInvariant()}, {result}, {command.SupportsProgress.ToString().ToLowerInvariant()}, {command.SupportsCancellation.ToString().ToLowerInvariant()}),");
        }
        sb.AppendLine("        ],");
        sb.AppendLine("        [");
        foreach (var map in model.Maps)
        {
            var valueDescriptor = map.ValueKind == ViewModelValueKind.Model
                ? $"{CSharpModelMetadataTypeName(ir, map.ValueModelName!)}.Descriptor"
                : "null";
            sb.AppendLine($"            new({map.Id}, {JsonSerializer.Serialize(map.Name)}, RustViewModelValueKind.{map.KeyKind}, RustViewModelValueKind.{map.ValueKind}, {valueDescriptor}),");
        }
        sb.AppendLine("        ]);");
        foreach (var collection in model.Collections.Where(collection => collection.Table is not null))
        {
            var table = collection.Table!;
            sb.AppendLine();
            sb.AppendLine($"    public static RustTableDescriptor {collection.Name}Table {{ get; }} = Create{collection.Name}Table();");
            sb.AppendLine();
            sb.AppendLine($"    public static IReadOnlyList<TableViewColumn> Create{collection.Name}TableColumns() =>");
            sb.AppendLine("    [");
            foreach (var column in table.Columns)
            {
                var limits = new StringBuilder();
                if (column.MinWidth is { } minWidth)
                    limits.Append($", MinWidth = {CSharpNullableDouble(minWidth)}");
                if (column.MaxWidth is { } maxWidth)
                    limits.Append($", MaxWidth = {CSharpNullableDouble(maxWidth)}");
                sb.AppendLine($"        new() {{ Header = {JsonSerializer.Serialize(column.Header)}, Width = {CSharpGridLength(column)}{limits}, CanUserResize = {column.Resizable.ToString().ToLowerInvariant()}, HorizontalContentAlignment = HorizontalAlignment.{column.HorizontalAlignment} }},");
            }
            sb.AppendLine("    ];");
            sb.AppendLine();
            sb.AppendLine($"    private static RustTableDescriptor Create{collection.Name}Table() => new(");
            sb.AppendLine("        [");
            foreach (var column in table.Columns)
            {
                sb.AppendLine($"            new({column.Id}, {JsonSerializer.Serialize(column.Name)}, {JsonSerializer.Serialize(column.Header)}, {JsonSerializer.Serialize(column.Path)}, {CSharpNullableDouble(column.Width)}, {column.Star.ToString().ToLowerInvariant()}, {column.Auto.ToString().ToLowerInvariant()}, {CSharpNullableDouble(column.MinWidth)}, {CSharpNullableDouble(column.MaxWidth)}, {column.Resizable.ToString().ToLowerInvariant()}, {column.Sortable.ToString().ToLowerInvariant()}, RustTableHorizontalAlignment.{column.HorizontalAlignment}),");
            }
            sb.AppendLine("        ],");
            sb.AppendLine(table.Selection is null
                ? "        null,"
                : $"        new({CSharpNullableString(table.Selection.SelectedIndexProperty)}, {CSharpNullableString(table.Selection.SelectedKeyProperty)}, {CSharpNullableString(table.Selection.RowKeyPath)}),");
            sb.AppendLine(table.Sort is null
                ? "        null);"
                : $"        new({JsonSerializer.Serialize(table.Sort.Command + "Command")}, {JsonSerializer.Serialize(table.Sort.Column)}, {JsonSerializer.Serialize(table.Sort.DirectionProperty)}));");
        }
        sb.AppendLine("}");
        return sb.ToString();
    }

    private static string EmitRegistry(ViewModelIr ir)
    {
        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine("using System;");
        sb.AppendLine("using Avalonia.Controls;");
        sb.AppendLine("using Avalonia.Rust;");
        sb.AppendLine("using Avalonia.Rust.Interop;");
        sb.AppendLine("using Avalonia.Threading;");
        sb.AppendLine();
        sb.AppendLine("namespace Avalonia.Host.Generated.ViewModels;");
        sb.AppendLine();
        sb.AppendLine("internal static class RustViewRegistry");
        sb.AppendLine("{");
        sb.AppendLine("    internal static Window Create(int viewId, IAvnRustViewModel model) => viewId switch");
        sb.AppendLine("    {");
        foreach (var view in ir.Views)
        {
            var model = ir.Models.Single(model => model.Name == view.Model);
            if (view.DynamicBindings)
            {
                sb.AppendLine(
                    $"        {view.Id} => CreateDynamic<global::{view.ManagedTypeName}>(model, global::{model.ManagedNamespace}.{model.Name}Metadata.Descriptor),");
            }
            else
            {
                sb.AppendLine($"        {view.Id} => new global::{view.ManagedTypeName}(model),");
            }
        }
        sb.AppendLine("        _ => throw new global::System.ArgumentOutOfRangeException(nameof(viewId)),");
        sb.AppendLine("    };");
        sb.AppendLine();
        sb.AppendLine("    private static TWindow CreateDynamic<TWindow>(");
        sb.AppendLine("        IAvnRustViewModel model,");
        sb.AppendLine("        RustViewModelDescriptor descriptor)");
        sb.AppendLine("        where TWindow : Window, new()");
        sb.AppendLine("    {");
        sb.AppendLine("        var adapter = new ReflectableRustViewModelAdapter(model, descriptor, Dispatch);");
        sb.AppendLine("        try");
        sb.AppendLine("        {");
        sb.AppendLine("            var window = new TWindow { DataContext = adapter };");
        sb.AppendLine("            window.Closed += (_, _) => adapter.Dispose();");
        sb.AppendLine("            return window;");
        sb.AppendLine("        }");
        sb.AppendLine("        catch");
        sb.AppendLine("        {");
        sb.AppendLine("            adapter.Dispose();");
        sb.AppendLine("            throw;");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine();
        sb.AppendLine("    private static void Dispatch(Action action)");
        sb.AppendLine("    {");
        sb.AppendLine("        if (Dispatcher.UIThread.CheckAccess())");
        sb.AppendLine("            action();");
        sb.AppendLine("        else");
        sb.AppendLine("            Dispatcher.UIThread.Invoke(action);");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        return sb.ToString();
    }

    private static void EmitRustModel(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        IEnumerable<ViewDefinition> views,
        bool externalConsumer)
    {
        var traitName = model.Name;
        var sinkName = $"{model.Name}Sink";
        var materialized = model.Collections.Where(collection => collection.Window is null).ToArray();
        var windowedCollections = model.Collections.Where(collection => collection.Window is not null).ToArray();
        var stringCollections = materialized.Where(collection => collection.ElementKind == ViewModelValueKind.String).ToArray();
        var modelCollections = materialized.Where(collection => collection.ElementKind == ViewModelValueKind.Model).ToArray();
        var numberCollections = materialized.Where(IsNumberCollection).ToArray();
        var trackedCommands = model.Commands.Where(command => command.SupportsCancellation || command.SupportsProgress).ToArray();
        sb.AppendLine("#[derive(Clone, Debug)]");
        sb.AppendLine($"pub struct {sinkName}(crate::view_model::ViewModelSink);");
        sb.AppendLine();
        if (model.RecentFiles is { } declaredRecent)
        {
            sb.AppendLine($"/// Declared capacity of the `{model.Name}` recent-file list.");
            sb.AppendLine($"pub const {Snake(model.Name).ToUpperInvariant()}_{Snake(declaredRecent.Collection).ToUpperInvariant()}_CAPACITY: usize = {declaredRecent.Capacity};");
            sb.AppendLine();
        }
        sb.AppendLine($"impl {sinkName} {{");
        foreach (var property in model.Properties)
            sb.AppendLine($"    pub fn {RustPropertySetterBody(property)}");
        foreach (var collection in stringCollections)
        {
            sb.AppendLine($"    pub fn add_{Snake(collection.Name)}(&self, value: impl AsRef<str>) -> crate::Result<()> {{ self.0.add_string({collection.Id}, value) }}");
            sb.AppendLine($"    pub fn insert_{Snake(collection.Name)}(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> {{ self.0.insert_string({collection.Id}, index, value) }}");
            sb.AppendLine($"    pub fn replace_{Snake(collection.Name)}(&self, index: i32, value: impl AsRef<str>) -> crate::Result<()> {{ self.0.replace_string({collection.Id}, index, value) }}");
        }
        foreach (var collection in modelCollections)
        {
            var elementTrait = collection.ElementModelName!;
            sb.AppendLine($"    pub fn add_{Snake(collection.Name)}(&self, value: impl {elementTrait}) -> crate::Result<()> {{ self.0.add_model({collection.Id}, {elementTrait}Dispatch {{ model: value }}) }}");
            sb.AppendLine($"    pub fn insert_{Snake(collection.Name)}(&self, index: i32, value: impl {elementTrait}) -> crate::Result<()> {{ self.0.insert_model({collection.Id}, index, {elementTrait}Dispatch {{ model: value }}) }}");
            sb.AppendLine($"    pub fn replace_{Snake(collection.Name)}(&self, index: i32, value: impl {elementTrait}) -> crate::Result<()> {{ self.0.replace_model({collection.Id}, index, {elementTrait}Dispatch {{ model: value }}) }}");
        }
        foreach (var collection in numberCollections)
        {
            var suffix = RustNumberSuffix(collection.ElementKind);
            var rustType = RustNumberType(collection.ElementKind);
            sb.AppendLine($"    pub fn add_{Snake(collection.Name)}(&self, value: {rustType}) -> crate::Result<()> {{ self.0.add_{suffix}({collection.Id}, value) }}");
            sb.AppendLine($"    pub fn insert_{Snake(collection.Name)}(&self, index: i32, value: {rustType}) -> crate::Result<()> {{ self.0.insert_{suffix}({collection.Id}, index, value) }}");
            sb.AppendLine($"    pub fn replace_{Snake(collection.Name)}(&self, index: i32, value: {rustType}) -> crate::Result<()> {{ self.0.replace_{suffix}({collection.Id}, index, value) }}");
        }
        if (numberCollections.Length > 0)
        {
            sb.AppendLine("    /// True when the attached host implements the scalar-number sink capability.");
            sb.AppendLine("    /// The reflectable (dynamic-binding) adapter deliberately does not.");
            sb.AppendLine("    pub fn supports_number_collections(&self) -> bool { self.0.supports_number_collections() }");
        }
        foreach (var collection in stringCollections)
        {
            sb.AppendLine($"    pub fn remove_{Snake(collection.Name)}(&self, index: i32) -> crate::Result<()> {{ self.0.remove_string_at({collection.Id}, index) }}");
            sb.AppendLine($"    pub fn move_{Snake(collection.Name)}(&self, from_index: i32, to_index: i32) -> crate::Result<()> {{ self.0.move_string_item({collection.Id}, from_index, to_index) }}");
            sb.AppendLine($"    pub fn clear_{Snake(collection.Name)}(&self) -> crate::Result<()> {{ self.0.clear_string_collection({collection.Id}) }}");
        }
        foreach (var collection in modelCollections)
        {
            sb.AppendLine($"    pub fn remove_{Snake(collection.Name)}(&self, index: i32) -> crate::Result<()> {{ self.0.remove_model_at({collection.Id}, index) }}");
            sb.AppendLine($"    pub fn move_{Snake(collection.Name)}(&self, from_index: i32, to_index: i32) -> crate::Result<()> {{ self.0.move_model_item({collection.Id}, from_index, to_index) }}");
            sb.AppendLine($"    pub fn clear_{Snake(collection.Name)}(&self) -> crate::Result<()> {{ self.0.clear_model_collection({collection.Id}) }}");
        }
        foreach (var collection in numberCollections)
        {
            sb.AppendLine($"    pub fn remove_{Snake(collection.Name)}(&self, index: i32) -> crate::Result<()> {{ self.0.remove_number_at({collection.Id}, index) }}");
            sb.AppendLine($"    pub fn move_{Snake(collection.Name)}(&self, from_index: i32, to_index: i32) -> crate::Result<()> {{ self.0.move_number_item({collection.Id}, from_index, to_index) }}");
            sb.AppendLine($"    pub fn clear_{Snake(collection.Name)}(&self) -> crate::Result<()> {{ self.0.clear_number_collection({collection.Id}) }}");
        }
        foreach (var command in model.Commands)
            sb.AppendLine($"    pub fn set_{Snake(command.Name)}_enabled(&self, enabled: bool) -> crate::Result<()> {{ self.0.set_command_enabled({command.Id}, enabled) }}");
        if (model.RecentFiles is { } recentFiles)
        {
            var collection = model.Collections.Single(candidate => candidate.Name == recentFiles.Collection);
            sb.AppendLine($"    /// Publishes the most-recently-used storage URIs into `{collection.Name}`.");
            sb.AppendLine($"    ///");
            sb.AppendLine($"    /// The list is bounded by its own capacity ({recentFiles.Capacity}), so this replaces a");
            sb.AppendLine($"    /// handful of entries rather than a data set; the generated menu derives");
            sb.AppendLine($"    /// each header from the URI and passes the URI back as the command parameter.");
            sb.AppendLine($"    pub fn publish_{Snake(recentFiles.Collection)}(&self, recent: &crate::RecentFileList) -> crate::Result<()> {{");
            sb.AppendLine($"        self.0.clear_string_collection({collection.Id})?;");
            sb.AppendLine($"        for uri in recent.entries() {{ self.0.add_string({collection.Id}, uri)?; }}");
            sb.AppendLine("        Ok(())");
            sb.AppendLine("    }");
        }
        foreach (var property in model.Properties.Where(property => property.Kind != ViewModelValueKind.Model))
            sb.AppendLine($"    pub fn set_{Snake(property.Name)}_error(&self, message: Option<&str>) -> crate::Result<()> {{ self.0.set_property_error({property.Id}, message) }}");
        EmitRustShapeSinkMethods(sb, ir, model, windowedCollections, trackedCommands);
        sb.AppendLine($"    /// Creates a worker-safe immutable update batch with a monotonic generation.");
        sb.AppendLine($"    pub fn batch(&self, generation: i64) -> {sinkName}Batch {{ {sinkName}Batch(crate::view_model::ViewModelBatch::new(generation)) }}");
        sb.AppendLine($"    pub fn submit_batch(&self, batch: {sinkName}Batch) -> crate::Result<crate::view_model::BatchCompletion> {{ self.0.submit_batch(batch.0) }}");
        sb.AppendLine("}");
        sb.AppendLine();
        EmitRustBatchBuilder(sb, ir, model, sinkName, stringCollections, modelCollections);
        sb.AppendLine($"pub trait {traitName}: Send + 'static {{");
        sb.AppendLine($"    fn attach(&mut self, sink: {sinkName}) -> crate::Result<()>;");
        sb.AppendLine("    fn detach(&mut self) -> crate::Result<()>;");
        foreach (var property in model.Properties.Where(property => property.Writable))
            sb.AppendLine($"    fn set_{Snake(property.Name)}(&mut self, value: {RustPropertyType(ir, property)}) -> crate::Result<()>;");
        foreach (var command in model.Commands)
        {
            var parameter = command.ParameterProperty is { } parameterProperty
                ? $", value: {RustOwnedType(model.Properties.Single(property => property.Name == parameterProperty).Kind)}"
                : AcceptsCommandParameter(model, command) ? ", value: String" : "";
            var token = command.SupportsCancellation ? ", token: crate::CancellationToken" : "";
            sb.AppendLine($"    fn {Snake(command.Name)}(&mut self{parameter}{token}) -> crate::Result<()>;");
        }
        foreach (var collection in windowedCollections)
        {
            sb.AppendLine($"    /// Realizes one page of `{collection.Name}`. Called on the runtime's dedicated");
            sb.AppendLine("    /// range thread, never on the UI thread, so it may take as long as the dataset needs.");
            sb.AppendLine($"    fn request_{Snake(collection.Name)}_range(&mut self, request: crate::RangeRequest) -> crate::Result<()>;");
        }
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine($"struct {traitName}Dispatch<T: {traitName}> {{ model: T }}");
        sb.AppendLine();
        sb.AppendLine($"impl<T: {traitName}> crate::view_model::DynamicViewModel for {traitName}Dispatch<T> {{");
        sb.AppendLine($"    fn attach(&mut self, sink: crate::view_model::ViewModelSink) -> crate::Result<()> {{ self.model.attach({sinkName}(sink)) }}");
        sb.AppendLine("    fn detach(&mut self) -> crate::Result<()> { self.model.detach() }");
        EmitRustPropertyDispatch(sb, model, ViewModelValueKind.String);
        EmitRustPropertyDispatch(sb, model, ViewModelValueKind.Integer);
        EmitRustPropertyDispatch(sb, model, ViewModelValueKind.Boolean);
        EmitRustPropertyDispatch(sb, model, ViewModelValueKind.Double);
        EmitRustCommandDispatch(sb, model, asyncCommands: false);
        EmitRustCommandDispatch(sb, model, asyncCommands: true);
        EmitRustTrackedCommandDispatch(sb, model);
        EmitRustRangeDispatch(sb, windowedCollections);
        sb.AppendLine("}");
        sb.AppendLine();
        foreach (var view in views)
        {
            if (externalConsumer)
                sb.AppendLine($"pub fn mount_{Snake(view.Name)}(scope: &crate::AppScope, model: impl {traitName}) -> crate::Result<()> {{ scope.mount_dynamic_view_model({view.Id}, {traitName}Dispatch {{ model }}) }}");
            else
                sb.AppendLine($"impl crate::AppScope {{ pub fn mount_{Snake(view.Name)}(&self, model: impl {traitName}) -> crate::Result<()> {{ self.mount_dynamic_view_model({view.Id}, {traitName}Dispatch {{ model }}) }} }}");
            if (view.Converters.Count > 0)
            {
                if (externalConsumer)
                    sb.AppendLine($"pub fn mount_{Snake(view.Name)}_with_converters<C: ValueConverters>(");
                else
                {
                    sb.AppendLine("impl crate::AppScope {");
                    sb.AppendLine($"    pub fn mount_{Snake(view.Name)}_with_converters<C: ValueConverters>(");
                    sb.AppendLine("        &self,");
                }
                if (externalConsumer)
                    sb.AppendLine("    scope: &crate::AppScope,");
                sb.AppendLine($"        model: impl {traitName},");
                sb.AppendLine($"        converters: C,");
                sb.AppendLine($"    ) -> crate::Result<()> {{");
                sb.AppendLine(externalConsumer
                    ? "        register_value_converters(scope, converters)?;"
                    : "        self.register_value_converters(converters)?;");
                sb.AppendLine(externalConsumer
                    ? $"        mount_{Snake(view.Name)}(scope, model)"
                    : $"        self.mount_{Snake(view.Name)}(model)");
                sb.AppendLine($"    }}");
                if (!externalConsumer)
                    sb.AppendLine("}");
            }
        }
        sb.AppendLine();
    }

    private static void EmitRustBatchBuilder(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        string sinkName,
        IEnumerable<ViewModelCollection> stringCollections,
        IEnumerable<ViewModelCollection> modelCollections)
    {
        sb.AppendLine($"pub struct {sinkName}Batch(crate::view_model::ViewModelBatch);");
        sb.AppendLine();
        sb.AppendLine($"impl {sinkName}Batch {{");
        foreach (var property in model.Properties)
        {
            var name = Snake(property.Name);
            switch (property.Kind)
            {
                case ViewModelValueKind.String:
                    if (property.Nullable)
                        sb.AppendLine($"    pub fn set_{name}(&mut self, value: Option<impl AsRef<str>>) {{ match value {{ Some(value) => self.0.push_string(1, {property.Id}, 0, value), None => self.0.push_null({property.Id}) }} }}");
                    else
                        sb.AppendLine($"    pub fn set_{name}(&mut self, value: impl AsRef<str>) {{ self.0.push_string(1, {property.Id}, 0, value); }}");
                    break;
                case ViewModelValueKind.Integer:
                    sb.AppendLine($"    pub fn set_{name}(&mut self, value: i64) {{ self.0.push_integer({property.Id}, value); }}");
                    break;
                case ViewModelValueKind.Enum:
                    sb.AppendLine($"    pub fn set_{name}(&mut self, value: {RustPropertyType(ir, property)}) {{ self.0.push_integer({property.Id}, value as i64); }}");
                    break;
                case ViewModelValueKind.Boolean:
                    sb.AppendLine($"    pub fn set_{name}(&mut self, value: bool) {{ self.0.push_boolean(3, {property.Id}, value); }}");
                    break;
                case ViewModelValueKind.Double:
                    sb.AppendLine($"    pub fn set_{name}(&mut self, value: f64) {{ self.0.push_double({property.Id}, value); }}");
                    break;
                case ViewModelValueKind.Model:
                    sb.AppendLine($"    pub fn set_{name}(&mut self, value: impl {property.ModelName}) {{ self.0.push_model(6, {property.Id}, 0, {property.ModelName}Dispatch {{ model: value }}); }}");
                    sb.AppendLine($"    pub fn clear_{name}(&mut self) {{ self.0.push_model_null({property.Id}); }}");
                    break;
            }
            sb.AppendLine($"    pub fn set_{name}_error(&mut self, message: impl AsRef<str>) {{ self.0.push_string(18, {property.Id}, 0, message); }}");
            sb.AppendLine($"    pub fn clear_{name}_error(&mut self) {{ self.0.push_clear_error({property.Id}); }}");
        }
        foreach (var collection in stringCollections)
        {
            var name = Snake(collection.Name);
            sb.AppendLine($"    pub fn add_{name}(&mut self, value: impl AsRef<str>) {{ self.0.push_string(7, {collection.Id}, 0, value); }}");
            sb.AppendLine($"    pub fn insert_{name}(&mut self, index: i32, value: impl AsRef<str>) {{ self.0.push_string(9, {collection.Id}, index, value); }}");
            sb.AppendLine($"    pub fn replace_{name}(&mut self, index: i32, value: impl AsRef<str>) {{ self.0.push_string(11, {collection.Id}, index, value); }}");
            sb.AppendLine($"    pub fn replace_{name}_snapshot<S: AsRef<str>>(&mut self, values: impl IntoIterator<Item = S>) {{ self.0.push_string_snapshot({collection.Id}, values); }}");
            sb.AppendLine($"    pub fn remove_{name}(&mut self, index: i32) {{ self.0.push_indices(13, {collection.Id}, index, 0); }}");
            sb.AppendLine($"    pub fn move_{name}(&mut self, from_index: i32, to_index: i32) {{ self.0.push_indices(14, {collection.Id}, from_index, to_index); }}");
            sb.AppendLine($"    pub fn clear_{name}(&mut self) {{ self.0.push_indices(19, {collection.Id}, 0, 0); }}");
        }
        foreach (var collection in modelCollections)
        {
            var name = Snake(collection.Name);
            sb.AppendLine($"    pub fn add_{name}(&mut self, value: impl {collection.ElementModelName}) {{ self.0.push_model(8, {collection.Id}, 0, {collection.ElementModelName}Dispatch {{ model: value }}); }}");
            sb.AppendLine($"    pub fn insert_{name}(&mut self, index: i32, value: impl {collection.ElementModelName}) {{ self.0.push_model(10, {collection.Id}, index, {collection.ElementModelName}Dispatch {{ model: value }}); }}");
            sb.AppendLine($"    pub fn replace_{name}(&mut self, index: i32, value: impl {collection.ElementModelName}) {{ self.0.push_model(12, {collection.Id}, index, {collection.ElementModelName}Dispatch {{ model: value }}); }}");
            sb.AppendLine($"    pub fn replace_{name}_snapshot<M: {collection.ElementModelName}>(&mut self, values: impl IntoIterator<Item = M>) {{ self.0.push_model_snapshot({collection.Id}, values.into_iter().map(|value| {collection.ElementModelName}Dispatch {{ model: value }})); }}");
            sb.AppendLine($"    pub fn remove_{name}(&mut self, index: i32) {{ self.0.push_model_indices(13, {collection.Id}, index, 0); }}");
            sb.AppendLine($"    pub fn move_{name}(&mut self, from_index: i32, to_index: i32) {{ self.0.push_model_indices(14, {collection.Id}, from_index, to_index); }}");
            sb.AppendLine($"    pub fn clear_{name}(&mut self) {{ self.0.push_model_clear({collection.Id}); }}");
        }
        foreach (var command in model.Commands)
            sb.AppendLine($"    pub fn set_{Snake(command.Name)}_enabled(&mut self, enabled: bool) {{ self.0.push_boolean(17, {command.Id}, enabled); }}");
        if (model.RecentFiles is { } recentFiles)
        {
            var recentCollection = model.Collections.Single(candidate => candidate.Name == recentFiles.Collection);
            sb.AppendLine($"    /// Stages the most-recently-used storage URIs as one `{recentCollection.Name}` snapshot.");
            sb.AppendLine($"    pub fn set_{Snake(recentFiles.Collection)}(&mut self, recent: &crate::RecentFileList) {{ self.0.push_string_snapshot({recentCollection.Id}, recent.entries()); }}");
        }
        sb.AppendLine("}");
        sb.AppendLine();
    }

    /// <summary>
    /// Builds the named per-property sink setter method body for the
    /// `impl {Model}Sink` block. Covers non-nullable scalars (unchanged
    /// shape), nullable strings (`Option&lt;T&gt;` publishing `set_null` for
    /// `None`), enums (cast to the wire's `Integer` transport), and nested
    /// view-model properties (always `Option`, routed through the generic
    /// `set_model`).
    /// </summary>
    private static string RustPropertySetterBody(ViewModelProperty property)
    {
        var name = Snake(property.Name);
        return property.Kind switch
        {
            ViewModelValueKind.Model =>
                $"set_{name}<M: {property.ModelName}>(&self, value: Option<M>) -> crate::Result<()> {{ self.0.set_model({property.Id}, value.map(|model| {property.ModelName}Dispatch {{ model }})) }}",
            ViewModelValueKind.Enum when property.Nullable =>
                throw new InvalidOperationException("Nullable enum properties are not supported."),
            ViewModelValueKind.Enum =>
                $"set_{name}(&self, value: {property.EnumName}) -> crate::Result<()> {{ self.0.set_integer({property.Id}, value as i64) }}",
            ViewModelValueKind.String when property.Nullable =>
                $"set_{name}(&self, value: Option<impl AsRef<str>>) -> crate::Result<()> {{ match value {{ Some(value) => self.0.set_string({property.Id}, value), None => self.0.set_null({property.Id}) }} }}",
            _ =>
                $"set_{name}(&self, value: {RustInputType(property.Kind)}) -> crate::Result<()> {{ self.0.set_{Snake(TransportSuffix(property.Kind))}({property.Id}, value) }}",
        };
    }

    private static string RustPropertyType(ViewModelIr ir, ViewModelProperty property) => property.Kind switch
    {
        ViewModelValueKind.Enum => property.EnumName!,
        _ => RustOwnedType(property.Kind),
    };

    private static void EmitRustConverters(StringBuilder sb, IReadOnlyList<ValueConverterDefinition> converters, bool externalConsumer)
    {
        if (converters.Count == 0)
            return;

        var anyParameterUsed = converters.Any(converter => converter.ParameterKind is not null);
        var parameterName = anyParameterUsed ? "parameter" : "_parameter";

        sb.AppendLine("pub trait ValueConverters: Send + Sync + 'static {");
        foreach (var converter in converters)
        {
            var forwardParameter = converter.ParameterKind is { } forwardParameterKind
                ? $", parameter: {RustOwnedType(forwardParameterKind)}"
                : "";
            sb.AppendLine($"    fn {Snake(converter.Name)}(&self, value: {RustOwnedType(converter.ValueKind)}{forwardParameter}) -> {RustOwnedType(converter.ResultKind)};");
            if (converter.SupportsConvertBack)
            {
                var backParameter = converter.ParameterKind is { } backParameterKind
                    ? $", parameter: {RustOwnedType(backParameterKind)}"
                    : "";
                sb.AppendLine($"    fn {Snake(converter.Name)}_back(&self, value: {RustOwnedType(converter.ResultKind)}{backParameter}) -> {RustOwnedType(converter.ValueKind)};");
            }
        }
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine("struct ValueConvertersDispatch<T: ValueConverters> { converters: T }");
        sb.AppendLine();
        sb.AppendLine("impl<T: ValueConverters> crate::value_converter::ValueConverterDispatch for ValueConvertersDispatch<T> {");
        sb.AppendLine("    fn convert(");
        sb.AppendLine("        &self,");
        sb.AppendLine("        converter_id: i32,");
        sb.AppendLine("        direction: crate::ConversionDirection,");
        sb.AppendLine("        value: crate::ScalarValue,");
        sb.AppendLine($"        {parameterName}: crate::ScalarValue,");
        sb.AppendLine("        _target_kind: crate::ScalarKind,");
        sb.AppendLine("        _culture: &str,");
        sb.AppendLine("    ) -> crate::Result<crate::ScalarValue> {");
        sb.AppendLine("        use crate::{ConversionDirection, ScalarValue};");
        sb.AppendLine("        match (converter_id, direction) {");
        foreach (var converter in converters)
        {
            sb.AppendLine($"            ({converter.Id}, ConversionDirection::Convert) => {{");
            sb.AppendLine($"                let value = {DecodeScalarMatch("value", converter.ValueKind, converter.Id, "converter-value")};");
            if (converter.ParameterKind is { } forwardParameterKind2)
                sb.AppendLine($"                let parameter = {DecodeScalarMatch(parameterName, forwardParameterKind2, converter.Id, "converter-parameter")};");
            var forwardCall = converter.ParameterKind is null
                ? $"self.converters.{Snake(converter.Name)}(value)"
                : $"self.converters.{Snake(converter.Name)}(value, parameter)";
            sb.AppendLine($"                Ok({EncodeScalarExpr(forwardCall, converter.ResultKind)})");
            sb.AppendLine("            }");
            if (converter.SupportsConvertBack)
            {
                sb.AppendLine($"            ({converter.Id}, ConversionDirection::ConvertBack) => {{");
                sb.AppendLine($"                let value = {DecodeScalarMatch("value", converter.ResultKind, converter.Id, "converter-value")};");
                if (converter.ParameterKind is { } backParameterKind2)
                    sb.AppendLine($"                let parameter = {DecodeScalarMatch(parameterName, backParameterKind2, converter.Id, "converter-parameter")};");
                var backCall = converter.ParameterKind is null
                    ? $"self.converters.{Snake(converter.Name)}_back(value)"
                    : $"self.converters.{Snake(converter.Name)}_back(value, parameter)";
                sb.AppendLine($"                Ok({EncodeScalarExpr(backCall, converter.ValueKind)})");
                sb.AppendLine("            }");
            }
        }
        sb.AppendLine("            _ => Err(crate::Error::InvalidViewModelMember { kind: \"converter\", id: converter_id }),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        sb.AppendLine();
        if (externalConsumer)
        {
            sb.AppendLine("pub fn register_value_converters(scope: &crate::AppScope, converters: impl ValueConverters) -> crate::Result<()> {");
            sb.AppendLine("    scope.register_value_converter_dispatch(ValueConvertersDispatch { converters })");
            sb.AppendLine("}");
        }
        else
        {
            sb.AppendLine("impl crate::AppScope {");
            sb.AppendLine("    pub fn register_value_converters(&self, converters: impl ValueConverters) -> crate::Result<()> {");
            sb.AppendLine("        self.register_value_converter_dispatch(ValueConvertersDispatch { converters })");
            sb.AppendLine("    }");
            sb.AppendLine("}");
        }
        sb.AppendLine();
    }

    private static string DecodeScalarMatch(string variable, ViewModelValueKind kind, int converterId, string what)
    {
        var pattern = kind switch
        {
            ViewModelValueKind.String => "ScalarValue::String(value) => value",
            ViewModelValueKind.Integer => "ScalarValue::Int64(value) => value",
            ViewModelValueKind.Boolean => "ScalarValue::Boolean(value) => value",
            ViewModelValueKind.Double => "ScalarValue::Double(value) => value",
            _ => throw new ArgumentOutOfRangeException(nameof(kind)),
        };
        return $"match {variable} {{ {pattern}, ScalarValue::Unset => return Ok(ScalarValue::Unset), ScalarValue::DoNothing => return Ok(ScalarValue::DoNothing), ScalarValue::Null => return Ok(ScalarValue::Null), _ => return Err(crate::Error::InvalidViewModelMember {{ kind: \"{what}\", id: {converterId} }}) }}";
    }

    private static string EncodeScalarExpr(string expression, ViewModelValueKind kind) => kind switch
    {
        ViewModelValueKind.String => $"ScalarValue::String({expression})",
        ViewModelValueKind.Integer => $"ScalarValue::Int64({expression})",
        ViewModelValueKind.Boolean => $"ScalarValue::Boolean({expression})",
        ViewModelValueKind.Double => $"ScalarValue::Double({expression})",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    private static string EmitCSharpConverter(ValueConverterDefinition converter)
    {
        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine("using Avalonia.Data.Converters;");
        sb.AppendLine("using Avalonia.Rust;");
        sb.AppendLine();
        sb.AppendLine($"namespace {converter.ManagedNamespace};");
        sb.AppendLine();
        sb.AppendLine($"public static class {converter.Name}Converter");
        sb.AppendLine("{");
        sb.AppendLine(
            $"    public static IValueConverter Instance {{ get; }} = new RustValueConverter({converter.Id}, {(converter.SupportsConvertBack ? "true" : "false")});");
        sb.AppendLine("}");
        return sb.ToString();
    }

    private static void EmitRustPropertyDispatch(
        StringBuilder sb,
        ViewModelDefinition model,
        ViewModelValueKind wireKind)
    {
        var transport = Snake(TransportSuffix(wireKind));
        var properties = model.Properties
            .Where(property => property.Writable && WireKind(property.Kind) == wireKind)
            .ToArray();
        var valueName = properties.Length == 0 ? "_value" : "value";
        sb.AppendLine($"    fn set_{transport}(&mut self, property_id: i32, {valueName}: {RustOwnedType(wireKind)}) -> crate::Result<()> {{");
        if (properties.Length == 0)
        {
            sb.AppendLine("        Err(crate::Error::InvalidViewModelMember { kind: \"property\", id: property_id })");
            sb.AppendLine("    }");
            return;
        }
        sb.AppendLine("        match property_id {");
        foreach (var property in properties)
        {
            if (property.Kind == ViewModelValueKind.Enum)
            {
                sb.AppendLine($"            {property.Id} => self.model.set_{Snake(property.Name)}({property.EnumName}::try_from(value).map_err(|_| crate::Error::InvalidViewModelMember {{ kind: \"property\", id: property_id }})?),");
            }
            else
            {
                sb.AppendLine($"            {property.Id} => self.model.set_{Snake(property.Name)}(value),");
            }
        }
        sb.AppendLine($"            _ => Err(crate::Error::InvalidViewModelMember {{ kind: \"property\", id: property_id }}),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
    }

    private static void EmitRustCommandDispatch(
        StringBuilder sb,
        ViewModelDefinition model,
        bool asyncCommands)
    {
        var method = asyncCommands ? "begin_async" : "execute";
        var commands = model.Commands.Where(command => command.IsAsync == asyncCommands).ToArray();
        var parameterName = commands.Any(command =>
                command.ParameterProperty is not null || AcceptsCommandParameter(model, command))
            ? "parameter"
            : "_parameter";
        sb.AppendLine($"    fn {method}(&mut self, command_id: i32, {parameterName}: Option<String>) -> crate::Result<()> {{");
        if (commands.Length == 0)
        {
            sb.AppendLine("        Err(crate::Error::InvalidViewModelMember { kind: \"command\", id: command_id })");
            sb.AppendLine("    }");
            return;
        }
        sb.AppendLine("        match command_id {");
        foreach (var command in commands)
        {
            var argument = RustCommandWireArgument(model, command, "command_id");
            var token = command.SupportsCancellation
                ? (argument.Length == 0 ? "crate::CancellationToken::none()" : ", crate::CancellationToken::none()")
                : "";
            sb.AppendLine($"            {command.Id} => self.model.{Snake(command.Name)}({argument}{token}),");
        }
        sb.AppendLine($"            _ => Err(crate::Error::InvalidViewModelMember {{ kind: \"command\", id: command_id }}),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
    }

    /// <summary>
    /// Emits the tracked-async entry point. Only cancellable commands need the
    /// real token; every other async command reuses the plain dispatch, so a
    /// producer never has two divergent code paths for the same command.
    /// </summary>
    private static void EmitRustTrackedCommandDispatch(StringBuilder sb, ViewModelDefinition model)
    {
        var cancellable = model.Commands.Where(command => command.SupportsCancellation).ToArray();
        if (cancellable.Length == 0)
            return;
        sb.AppendLine("    fn begin_async_tracked(&mut self, command_id: i32, parameter: Option<String>, token: crate::CancellationToken) -> crate::Result<()> {");
        sb.AppendLine("        match command_id {");
        foreach (var command in cancellable)
        {
            var argument = RustCommandWireArgument(model, command, "command_id");
            var call = argument.Length == 0 ? "token" : $"{argument}, token";
            sb.AppendLine($"            {command.Id} => self.model.{Snake(command.Name)}({call}),");
        }
        sb.AppendLine("            _ => self.begin_async(command_id, parameter),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
    }

    /// <summary>
    /// Emits the range entry point. Only models declaring a windowed
    /// collection implement it; every other model keeps the trait default,
    /// which rejects the request explicitly rather than silently succeeding.
    /// </summary>
    private static void EmitRustRangeDispatch(
        StringBuilder sb,
        IReadOnlyList<ViewModelCollection> windowedCollections)
    {
        if (windowedCollections.Count == 0)
            return;
        sb.AppendLine("    fn request_range(&mut self, request: crate::RangeRequest) -> crate::Result<()> {");
        sb.AppendLine("        match request.collection_id {");
        foreach (var collection in windowedCollections)
            sb.AppendLine($"            {collection.Id} => self.model.request_{Snake(collection.Name)}_range(request),");
        sb.AppendLine("            _ => Err(crate::Error::InvalidViewModelMember { kind: \"collection\", id: request.collection_id }),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
    }

    /// <summary>
    /// Emits the sink's named stage 30 APIs: keyed map mutation, async
    /// progress/result/running publication and windowed range publication.
    /// Application authors never see a map ID, a key encoding, or a range
    /// batch's wire kind.
    /// </summary>
    private static void EmitRustShapeSinkMethods(
        StringBuilder sb,
        ViewModelIr ir,
        ViewModelDefinition model,
        IReadOnlyList<ViewModelCollection> windowedCollections,
        IReadOnlyList<ViewModelCommand> trackedCommands)
    {
        if (model.Maps.Count > 0 || windowedCollections.Count > 0 || trackedCommands.Count > 0 ||
            model.Commands.Any(command => command.ResultModelName is not null))
        {
            sb.AppendLine("    /// True when the attached host implements the stage 30 sink capability.");
            sb.AppendLine("    /// The reflectable (dynamic-binding) adapter deliberately does not.");
            sb.AppendLine("    pub fn supports_richer_shapes(&self) -> bool { self.0.supports_richer_shapes() }");
        }
        foreach (var map in model.Maps)
        {
            var name = Snake(map.Name);
            var key = RustMapKeyType(map);
            switch (map.ValueKind)
            {
                case ViewModelValueKind.String:
                    sb.AppendLine($"    pub fn set_{name}(&self, key: {key}, value: impl AsRef<str>) -> crate::Result<()> {{ self.0.map_set_string({map.Id}, key.into(), value) }}");
                    break;
                case ViewModelValueKind.Integer:
                    sb.AppendLine($"    pub fn set_{name}(&self, key: {key}, value: i64) -> crate::Result<()> {{ self.0.map_set_integer({map.Id}, key.into(), value) }}");
                    break;
                case ViewModelValueKind.Boolean:
                    sb.AppendLine($"    pub fn set_{name}(&self, key: {key}, value: bool) -> crate::Result<()> {{ self.0.map_set_boolean({map.Id}, key.into(), value) }}");
                    break;
                case ViewModelValueKind.Double:
                    sb.AppendLine($"    pub fn set_{name}(&self, key: {key}, value: f64) -> crate::Result<()> {{ self.0.map_set_double({map.Id}, key.into(), value) }}");
                    break;
                case ViewModelValueKind.Model:
                    sb.AppendLine($"    pub fn set_{name}(&self, key: {key}, value: impl {map.ValueModelName}) -> crate::Result<()> {{ self.0.map_set_model({map.Id}, key.into(), {map.ValueModelName}Dispatch {{ model: value }}) }}");
                    break;
            }
            sb.AppendLine($"    pub fn remove_{name}(&self, key: {key}) -> crate::Result<()> {{ self.0.map_remove({map.Id}, key.into()) }}");
            sb.AppendLine($"    pub fn clear_{name}(&self) -> crate::Result<()> {{ self.0.map_clear({map.Id}) }}");
        }
        foreach (var command in trackedCommands)
        {
            var name = Snake(command.Name);
            if (command.SupportsProgress)
            {
                sb.AppendLine($"    pub fn set_{name}_progress(&self, value: Option<f64>, message: Option<&str>) -> crate::Result<()> {{ self.0.set_command_progress({command.Id}, value, message) }}");
            }
            sb.AppendLine($"    pub fn set_{name}_running(&self, running: bool) -> crate::Result<()> {{ self.0.set_command_running({command.Id}, running) }}");
            if (command.SupportsCancellation)
            {
                sb.AppendLine($"    /// Claims the single terminal transition of one `{command.Name}` invocation.");
                sb.AppendLine($"    /// Returns false when success, failure or cancellation already claimed it.");
                sb.AppendLine($"    pub fn claim_{name}_completion(&self, token: &crate::CancellationToken) -> bool {{ self.0.claim_completion({command.Id}, token) }}");
            }
        }
        foreach (var command in model.Commands.Where(command => command.ResultModelName is not null))
        {
            var name = Snake(command.Name);
            sb.AppendLine($"    pub fn set_{name}_result(&self, value: impl {command.ResultModelName}) -> crate::Result<()> {{ self.0.set_command_result({command.Id}, Some({command.ResultModelName}Dispatch {{ model: value }})) }}");
            sb.AppendLine($"    pub fn clear_{name}_result(&self) -> crate::Result<()> {{ self.0.clear_command_result({command.Id}) }}");
        }
        foreach (var collection in windowedCollections)
        {
            var name = Snake(collection.Name);
            sb.AppendLine($"    /// Republishes `{collection.Name}`'s dataset identity, invalidating every realized page.");
            sb.AppendLine($"    pub fn reset_{name}(&self, generation: i64, total_count: i64) -> crate::Result<()> {{ self.0.publish_range_reset({collection.Id}, generation, total_count) }}");
            sb.AppendLine($"    /// Starts a page for `{collection.Name}` at the currently published generation.");
            sb.AppendLine($"    pub fn {name}_page(&self, offset: i64) -> Option<crate::RangeBatch> {{ self.0.range_batch({collection.Id}, offset) }}");
            if (collection.ElementKind == ViewModelValueKind.Model)
            {
                sb.AppendLine($"    pub fn push_{name}_row(&self, page: &mut crate::RangeBatch, value: impl {collection.ElementModelName}) {{ self.0.push_range_model(page, {collection.ElementModelName}Dispatch {{ model: value }}); }}");
            }
            else
            {
                sb.AppendLine($"    pub fn push_{name}_row(&self, page: &mut crate::RangeBatch, value: impl AsRef<str>) {{ let _ = self; page.push_text(value); }}");
            }
            sb.AppendLine($"    pub fn publish_{name}_page(&self, page: crate::RangeBatch) -> crate::Result<crate::view_model::BatchCompletion> {{ self.0.publish_range(page) }}");
            sb.AppendLine($"    /// Re-requests realized `{collection.Name}` pages at the current generation (live values, no adapter churn).");
            sb.AppendLine($"    pub fn refresh_{name}(&self) -> crate::Result<()> {{ self.0.publish_range_invalidate({collection.Id}) }}");
        }
    }

    private static string RustMapKeyType(ViewModelMap map) =>
        map.KeyKind == ViewModelValueKind.String ? "impl Into<crate::MapKey>" : "i64";

    private static string RustNumberSuffix(ViewModelValueKind elementKind) =>
        elementKind == ViewModelValueKind.Integer ? "integer" : "double";

    private static string RustNumberType(ViewModelValueKind elementKind) =>
        elementKind == ViewModelValueKind.Integer ? "i64" : "f64";

    private static readonly ViewModelValueKind[] ScalarWireKinds =
    [
        ViewModelValueKind.String,
        ViewModelValueKind.Integer,
        ViewModelValueKind.Boolean,
        ViewModelValueKind.Double,
    ];

    /// <summary>
    /// The scalar transport an application-facing <see cref="ViewModelValueKind"/>
    /// rides on. Only <see cref="ViewModelValueKind.Enum"/> maps to a
    /// different wire kind (<see cref="ViewModelValueKind.Integer"/>); every
    /// other scalar kind is its own wire kind. <see cref="ViewModelValueKind.Model"/>
    /// has no scalar wire representation (it uses `SetModel`/`AddModel`/etc.)
    /// and must never be passed here.
    /// </summary>
    private static ViewModelValueKind WireKind(ViewModelValueKind kind) => kind switch
    {
        ViewModelValueKind.Enum => ViewModelValueKind.Integer,
        _ => kind,
    };

    private static string CSharpType(ViewModelIr ir, ViewModelProperty property) => property.Kind switch
    {
        ViewModelValueKind.String => property.Nullable ? "string?" : "string",
        ViewModelValueKind.Integer => "long",
        ViewModelValueKind.Boolean => "bool",
        ViewModelValueKind.Double => "double",
        ViewModelValueKind.Enum => CSharpEnumTypeName(ir, property.EnumName!),
        ViewModelValueKind.Model => $"{CSharpModelAdapterTypeName(ir, property.ModelName!)}?",
        _ => throw new ArgumentOutOfRangeException(nameof(property)),
    };

    private static string CSharpElementType(ViewModelIr ir, ViewModelValueKind elementKind, string? elementModelName) => elementKind switch
    {
        ViewModelValueKind.String => "string",
        ViewModelValueKind.Integer => "long",
        ViewModelValueKind.Double => "double",
        ViewModelValueKind.Model => CSharpModelAdapterTypeName(ir, elementModelName!),
        _ => throw new ArgumentOutOfRangeException(nameof(elementKind)),
    };

    /// <summary>
    /// Whether a collection projects a scalar number list, which is the shape
    /// carried by <c>IAvnRustVmSink5</c>. Strings keep riding the already
    /// published v1/v2 transport and nested models the v2 model transport.
    /// </summary>
    private static bool IsNumberCollection(ViewModelCollection collection) =>
        collection.ElementKind is ViewModelValueKind.Integer or ViewModelValueKind.Double;

    private static string CSharpScalarType(ViewModelValueKind wireKind) => wireKind switch
    {
        ViewModelValueKind.String => "string",
        ViewModelValueKind.Integer => "long",
        ViewModelValueKind.Boolean => "bool",
        ViewModelValueKind.Double => "double",
        _ => throw new ArgumentOutOfRangeException(nameof(wireKind)),
    };

    private static string CSharpTransportType(ViewModelValueKind wireKind) =>
        wireKind == ViewModelValueKind.Boolean ? "int" : CSharpScalarType(wireKind) + (wireKind == ViewModelValueKind.String ? "?" : "");

    private static bool IsTableSortCommand(ViewModelDefinition model, ViewModelCommand command) =>
        model.Collections.Any(collection => string.Equals(collection.Table?.Sort?.Command, command.Name, StringComparison.Ordinal));

    /// <summary>
    /// Whether a command takes its argument from the caller's
    /// <c>CommandParameter</c> rather than from a declared parameter property.
    /// Table sorting was the first such command; a recent-file activation is
    /// the second, because the chosen URI is a property of the clicked menu
    /// item, not of the model.
    /// </summary>
    /// <summary>
    /// Formats the UTF-16 command argument the nano-COM <c>Execute</c>/<c>BeginAsync</c>
    /// vtable carries. Declared parameter properties may be any scalar; they
    /// ride the existing string slot with invariant formatting.
    /// </summary>
    private static string CSharpCommandWireArgument(ViewModelDefinition model, ViewModelCommand command)
    {
        if (command.ParameterProperty is { } name)
        {
            var kind = model.Properties.Single(property => property.Name == name).Kind;
            var fallback = kind switch
            {
                ViewModelValueKind.String => name,
                ViewModelValueKind.Integer => $"{name}.ToString(CultureInfo.InvariantCulture)",
                ViewModelValueKind.Double => $"{name}.ToString(\"R\", CultureInfo.InvariantCulture)",
                ViewModelValueKind.Boolean => $"{name} ? \"true\" : \"false\"",
                _ => throw new InvalidOperationException($"Command '{model.Name}.{command.Name}' has an unsupported parameter kind '{kind}'."),
            };
            // A non-null CommandParameter (row button, menu item) wins over the
            // model property so Pin/Kill on a row bind {Binding Pid} instead of
            // the selected row. The Execute vtable is still one UTF-16 string.
            return $"CommandArgumentOrFallback(parameter, {fallback})";
        }

        return AcceptsCommandParameter(model, command) ? "parameter as string" : "null";
    }

    /// <summary>
    /// Parses the UTF-16 command argument into the generated Rust trait's
    /// owned parameter type. Empty when the command takes no value.
    /// </summary>
    private static string RustCommandWireArgument(ViewModelDefinition model, ViewModelCommand command, string commandIdExpr)
    {
        if (command.ParameterProperty is { } name)
        {
            var kind = model.Properties.Single(property => property.Name == name).Kind;
            if (kind == ViewModelValueKind.String)
                return "parameter.unwrap_or_default()";
            var rustType = RustOwnedType(kind);
            return $"parameter.as_deref().unwrap_or(\"\").parse::<{rustType}>().map_err(|_| crate::Error::InvalidViewModelMember {{ kind: \"command\", id: {commandIdExpr} }})?";
        }

        return AcceptsCommandParameter(model, command) ? "parameter.unwrap_or_default()" : "";
    }

    private static bool AcceptsCommandParameter(ViewModelDefinition model, ViewModelCommand command) =>
        IsTableSortCommand(model, command) ||
        string.Equals(model.RecentFiles?.ActivateCommand, command.Name, StringComparison.Ordinal);

    private static string CSharpNullableString(string? value) =>
        value is null ? "null" : JsonSerializer.Serialize(value);

    /// <summary>
    /// Serializes a literal for C# source with relaxed escaping, so an ordinary
    /// character such as <c>+</c> in a gesture or an access-key <c>_</c> in a
    /// header survives readably instead of becoming a <c>\u</c> escape.
    /// </summary>
    private static string CSharpString(string? value) =>
        value is null ? "null" : JsonSerializer.Serialize(value, RelaxedStrings);

    private static readonly JsonSerializerOptions RelaxedStrings = new()
    {
        Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    private static string CSharpNullableDouble(double? value) =>
        value is null ? "null" : value.Value.ToString("R", CultureInfo.InvariantCulture) + "D";

    private static string CSharpGridLength(ViewModelTableColumn column)
    {
        if (column.Auto)
            return "global::Avalonia.Controls.GridLength.Auto";
        if (column.Star)
            return "new global::Avalonia.Controls.GridLength(1D, global::Avalonia.Controls.GridUnitType.Star)";
        return $"new global::Avalonia.Controls.GridLength({CSharpNullableDouble(column.Width)!.Replace("null", "0D", StringComparison.Ordinal)}, global::Avalonia.Controls.GridUnitType.Pixel)";
    }

    private static string CSharpEnumTypeName(ViewModelIr ir, string enumName)
    {
        var enumDefinition = ir.Enums.Single(candidate => candidate.Name == enumName);
        return $"global::{enumDefinition.ManagedNamespace}.{enumDefinition.Name}";
    }

    private static string CSharpModelAdapterTypeName(ViewModelIr ir, string modelName)
    {
        var nestedModel = ir.Models.Single(candidate => candidate.Name == modelName);
        return $"global::{nestedModel.ManagedNamespace}.{nestedModel.Name}Adapter";
    }

    private static string CSharpModelMetadataTypeName(ViewModelIr ir, string modelName)
    {
        var nestedModel = ir.Models.Single(candidate => candidate.Name == modelName);
        return $"global::{nestedModel.ManagedNamespace}.{nestedModel.Name}Metadata";
    }

    private static string CSharpInitial(ViewModelIr ir, ViewModelProperty property) => property.Kind switch
    {
        ViewModelValueKind.String => property.Nullable
            ? (property.InitialString is null ? "null" : JsonSerializer.Serialize(property.InitialString))
            : JsonSerializer.Serialize(property.InitialString ?? ""),
        ViewModelValueKind.Integer => (property.InitialInteger ?? 0).ToString(CultureInfo.InvariantCulture) + "L",
        ViewModelValueKind.Boolean => (property.InitialBoolean ?? false) ? "true" : "false",
        ViewModelValueKind.Double => (property.InitialDouble ?? 0).ToString("R", CultureInfo.InvariantCulture) + "D",
        ViewModelValueKind.Enum => $"({CSharpEnumTypeName(ir, property.EnumName!)})({(property.InitialInteger ?? 0).ToString(CultureInfo.InvariantCulture)}L)",
        ViewModelValueKind.Model => "null",
        _ => throw new ArgumentOutOfRangeException(),
    };

    private static string CSharpToTransport(ViewModelProperty property, string value) => property.Kind switch
    {
        ViewModelValueKind.Boolean => $"({value} ? 1 : 0)",
        ViewModelValueKind.Enum => $"(long){value}",
        _ => value,
    };

    private static string CSharpAcceptedValue(ViewModelProperty property, string value) =>
        property.Kind == ViewModelValueKind.String && !property.Nullable
            ? $"{value} ?? \"\""
            : value;

    private static string CSharpFromTransport(ViewModelIr ir, ViewModelProperty property, string value) => property.Kind switch
    {
        ViewModelValueKind.String => property.Nullable ? value : $"{value} ?? \"\"",
        ViewModelValueKind.Boolean => $"{value} != 0",
        ViewModelValueKind.Enum => $"({CSharpEnumTypeName(ir, property.EnumName!)}){value}",
        _ => value,
    };

    private static string TransportSuffix(ViewModelValueKind wireKind) => wireKind switch
    {
        ViewModelValueKind.String => "String",
        ViewModelValueKind.Integer => "Integer",
        ViewModelValueKind.Boolean => "Boolean",
        ViewModelValueKind.Double => "Double",
        _ => throw new ArgumentOutOfRangeException(nameof(wireKind)),
    };

    private static string RustInputType(ViewModelValueKind kind) =>
        kind == ViewModelValueKind.String ? "impl AsRef<str>" : RustOwnedType(kind);

    private static string RustOwnedType(ViewModelValueKind kind) => kind switch
    {
        ViewModelValueKind.String => "String",
        ViewModelValueKind.Integer => "i64",
        ViewModelValueKind.Boolean => "bool",
        ViewModelValueKind.Double => "f64",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    private static string EmitCSharpEnum(ViewModelEnumDefinition enumDefinition)
    {
        var sb = new StringBuilder();
        sb.AppendLine("// <auto-generated />");
        sb.AppendLine("#nullable enable");
        sb.AppendLine();
        sb.AppendLine($"namespace {enumDefinition.ManagedNamespace};");
        sb.AppendLine();
        sb.AppendLine($"public enum {enumDefinition.Name} : long");
        sb.AppendLine("{");
        foreach (var member in enumDefinition.Members)
            sb.AppendLine($"    {member.Name} = {member.Value},");
        sb.AppendLine("}");
        return sb.ToString();
    }

    private static void EmitRustEnum(StringBuilder sb, ViewModelEnumDefinition enumDefinition)
    {
        sb.AppendLine("#[repr(i64)]");
        sb.AppendLine("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
        sb.AppendLine($"pub enum {enumDefinition.Name} {{");
        foreach (var member in enumDefinition.Members)
            sb.AppendLine($"    {member.Name} = {member.Value},");
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine($"impl std::convert::TryFrom<i64> for {enumDefinition.Name} {{");
        sb.AppendLine("    type Error = ();");
        sb.AppendLine("    fn try_from(value: i64) -> std::result::Result<Self, ()> {");
        sb.AppendLine("        match value {");
        foreach (var member in enumDefinition.Members)
            sb.AppendLine($"            {member.Value} => Ok(Self::{member.Name}),");
        sb.AppendLine("            _ => Err(()),");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        sb.AppendLine();
    }

    private static string Lower(string value) =>
        char.ToLowerInvariant(value[0]) + value[1..];

    private static string Snake(string value)
    {
        var sb = new StringBuilder();
        for (var index = 0; index < value.Length; index++)
        {
            var character = value[index];
            if (char.IsUpper(character) && index > 0)
                sb.Append('_');
            sb.Append(char.ToLowerInvariant(character));
        }
        return sb.ToString();
    }
}
