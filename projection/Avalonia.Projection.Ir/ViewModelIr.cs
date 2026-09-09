using System.Text.Json;
using System.Text.Json.Serialization;

namespace Avalonia.Projection.Ir;

public sealed class ViewModelIr
{
    public const int CurrentVersion = 5;

    /// <summary>The first schema version that accepts stage 30 richer data shapes.</summary>
    public const int RicherDataShapesVersion = 4;

    /// <summary>
    /// The first schema version that accepts stage 31 command surfaces: menus,
    /// keyboard accelerators, recent-file lists and declared display paths.
    /// </summary>
    public const int CommandSurfaceVersion = 5;

    public int Version { get; init; } = CurrentVersion;
    public IReadOnlyList<ViewModelEnumDefinition> Enums { get; init; } = [];
    public IReadOnlyList<ViewModelDefinition> Models { get; init; } = [];
    public IReadOnlyList<ViewDefinition> Views { get; init; } = [];
    public IReadOnlyList<ValueConverterDefinition> Converters { get; init; } = [];

    public static ViewModelIr FromJson(string json)
    {
        var value = JsonSerializer.Deserialize<ViewModelIr>(json, JsonOptions)
            ?? throw new InvalidOperationException("View-model IR deserialized to null.");
        value.Validate();
        return value;
    }

    public string ToJson()
    {
        Validate();
        return JsonSerializer.Serialize(this, JsonOptions);
    }

    public void Validate()
    {
        if (Version is not (2 or 3 or 4 or CurrentVersion))
            throw new InvalidOperationException($"Unsupported view-model IR version {Version}.");
        if (Version == 2 && Models.Any(model => model.Collections.Any(collection => collection.Table is not null)))
            throw new InvalidOperationException("View-model IR version 2 does not support table metadata; upgrade to version 3.");
        if (Version < RicherDataShapesVersion && Models.Any(HasRicherDataShapes))
            throw new InvalidOperationException(
                $"View-model IR version {Version} does not support keyed maps, windowed collections, tree metadata, structured command results, progress or cancellation; upgrade to version {RicherDataShapesVersion}.");
        if (Version < CommandSurfaceVersion && Models.Any(HasCommandSurfaces))
            throw new InvalidOperationException(
                $"View-model IR version {Version} does not support menus, keyboard accelerators, recent-file lists or display paths; upgrade to version {CommandSurfaceVersion}.");
        EnsurePositiveIds(Enums, enumDefinition => enumDefinition.Id, "enum");
        EnsurePositiveIds(Models, model => model.Id, "model");
        EnsurePositiveIds(Views, view => view.Id, "view");
        EnsurePositiveIds(Converters, converter => converter.Id, "converter");
        EnsureUnique(Enums, enumDefinition => enumDefinition.Id, "enum ID");
        EnsureUnique(Enums, enumDefinition => enumDefinition.Name, "enum name");
        EnsureUnique(Models, model => model.Id, "model ID");
        EnsureUnique(Models, model => model.Name, "model name");
        EnsureUnique(Views, view => view.Id, "view ID");
        EnsureUnique(Views, view => view.Name, "view name");
        EnsureUnique(Converters, converter => converter.Id, "converter ID");
        EnsureUnique(Converters, converter => converter.Name, "converter name");

        var modelNames = Models.Select(model => model.Name).ToHashSet(StringComparer.Ordinal);
        var enumsByName = Enums.ToDictionary(enumDefinition => enumDefinition.Name, StringComparer.Ordinal);
        var converterNames = Converters.Select(converter => converter.Name).ToHashSet(StringComparer.Ordinal);

        foreach (var enumDefinition in Enums)
        {
            EnsureIdentifier(enumDefinition.Name, "Enum name");
            EnsureNotBlank(enumDefinition.ManagedNamespace, $"Managed namespace for enum '{enumDefinition.Name}'");
            if (enumDefinition.Members.Count == 0)
                throw new InvalidOperationException($"Enum '{enumDefinition.Name}' must declare at least one member.");
            EnsureUnique(enumDefinition.Members, member => member.Name, $"{enumDefinition.Name} member name");
            EnsureUnique(enumDefinition.Members, member => member.Value, $"{enumDefinition.Name} member value");
            foreach (var member in enumDefinition.Members)
                EnsureIdentifier(member.Name, $"Member name in enum '{enumDefinition.Name}'");
        }

        foreach (var view in Views)
        {
            EnsureIdentifier(view.Name, "View name");
            EnsureNotBlank(view.ManagedTypeName, $"Managed type for view '{view.Name}'");
            if (!modelNames.Contains(view.Model))
                throw new InvalidOperationException(
                    $"View '{view.Name}' references unknown model '{view.Model}'.");
            foreach (var converter in view.Converters)
            {
                if (!converterNames.Contains(converter))
                    throw new InvalidOperationException(
                        $"View '{view.Name}' references unknown converter '{converter}'.");
            }
        }

        foreach (var converter in Converters)
        {
            EnsureIdentifier(converter.Name, "Converter name");
            EnsureNotBlank(converter.ManagedNamespace, $"Managed namespace for converter '{converter.Name}'");
            if (!IsScalarKind(converter.ValueKind))
                throw new InvalidOperationException(
                    $"Converter '{converter.Name}' has an unsupported value kind '{converter.ValueKind}'; converters only support string/integer/boolean/double.");
            if (!IsScalarKind(converter.ResultKind))
                throw new InvalidOperationException(
                    $"Converter '{converter.Name}' has an unsupported result kind '{converter.ResultKind}'; converters only support string/integer/boolean/double.");
            if (converter.ParameterKind is { } parameterKind && (!Enum.IsDefined(parameterKind) || !IsScalarKind(parameterKind)))
                throw new InvalidOperationException(
                    $"Converter '{converter.Name}' has an invalid parameter kind.");
        }

        foreach (var model in Models)
        {
            EnsureIdentifier(model.Name, "Model name");
            EnsureNotBlank(model.ManagedNamespace, $"Managed namespace for model '{model.Name}'");
            EnsurePositiveIds(model.Properties, property => property.Id, $"{model.Name} property");
            EnsurePositiveIds(model.Collections, collection => collection.Id, $"{model.Name} collection");
            EnsurePositiveIds(model.Maps, map => map.Id, $"{model.Name} map");
            EnsurePositiveIds(model.Commands, command => command.Id, $"{model.Name} command");
            EnsurePositiveIds(model.Menus, menu => menu.Id, $"{model.Name} menu");
            EnsureUnique(model.Properties, property => property.Id, $"{model.Name} property ID");
            EnsureUnique(model.Properties, property => property.Name, $"{model.Name} property name");
            EnsureUnique(model.Collections, collection => collection.Id, $"{model.Name} collection ID");
            EnsureUnique(model.Collections, collection => collection.Name, $"{model.Name} collection name");
            EnsureUnique(model.Maps, map => map.Id, $"{model.Name} map ID");
            EnsureUnique(model.Maps, map => map.Name, $"{model.Name} map name");
            EnsureUnique(model.Commands, command => command.Id, $"{model.Name} command ID");
            EnsureUnique(model.Commands, command => command.Name, $"{model.Name} command name");
            EnsureUnique(model.Menus, menu => menu.Id, $"{model.Name} menu ID");
            EnsureUnique(model.Menus, menu => menu.Name, $"{model.Name} menu name");
            EnsureUnique(
                model.Properties.Select(property => property.Name)
                    .Concat(model.Collections.Select(collection => collection.Name))
                    .Concat(model.Maps.Select(map => map.Name)),
                name => name,
                $"{model.Name} member name");

            var properties = model.Properties
                .ToDictionary(property => property.Name, StringComparer.Ordinal);
            foreach (var property in model.Properties)
            {
                EnsureIdentifier(property.Name, $"Property name in model '{model.Name}'");
                ValidateInitialValue(model, property);
                ValidateEnumProperty(model, property, enumsByName);
                ValidateModelProperty(model, property, modelNames);
            }
            foreach (var collection in model.Collections)
            {
                EnsureIdentifier(collection.Name, $"Collection name in model '{model.Name}'");
                switch (collection.ElementKind)
                {
                    case ViewModelValueKind.String:
                    case ViewModelValueKind.Integer:
                    case ViewModelValueKind.Double:
                        if (collection.ElementModelName is not null)
                            throw new InvalidOperationException(
                                $"Collection '{model.Name}.{collection.Name}' declares 'elementModelName' but is not a nested view-model collection.");
                        break;
                    case ViewModelValueKind.Model:
                        if (collection.ElementModelName is null)
                            throw new InvalidOperationException(
                                $"Collection '{model.Name}.{collection.Name}' must declare 'elementModelName'.");
                        if (!modelNames.Contains(collection.ElementModelName))
                            throw new InvalidOperationException(
                                $"Collection '{model.Name}.{collection.Name}' references unknown model '{collection.ElementModelName}'.");
                        break;
                    default:
                        throw new InvalidOperationException(
                            $"Collection '{model.Name}.{collection.Name}' must contain strings, integers, doubles or nested view models.");
                }
                ValidateTable(model, collection, properties, Models);
                ValidateWindow(model, collection);
                ValidateTree(model, collection, Models);
            }
            foreach (var map in model.Maps)
            {
                EnsureIdentifier(map.Name, $"Map name in model '{model.Name}'");
                if (map.KeyKind is not (ViewModelValueKind.String or ViewModelValueKind.Integer))
                    throw new InvalidOperationException(
                        $"Map '{model.Name}.{map.Name}' must declare a string or integer key kind.");
                switch (map.ValueKind)
                {
                    case ViewModelValueKind.String:
                    case ViewModelValueKind.Integer:
                    case ViewModelValueKind.Boolean:
                    case ViewModelValueKind.Double:
                        if (map.ValueModelName is not null)
                            throw new InvalidOperationException(
                                $"Map '{model.Name}.{map.Name}' declares 'valueModelName' but is not a nested view-model map.");
                        break;
                    case ViewModelValueKind.Model:
                        if (map.ValueModelName is null)
                            throw new InvalidOperationException(
                                $"Map '{model.Name}.{map.Name}' must declare 'valueModelName'.");
                        if (!modelNames.Contains(map.ValueModelName))
                            throw new InvalidOperationException(
                                $"Map '{model.Name}.{map.Name}' references unknown model '{map.ValueModelName}'.");
                        break;
                    default:
                        throw new InvalidOperationException(
                            $"Map '{model.Name}.{map.Name}' has an unsupported value kind '{map.ValueKind}'.");
                }
            }
            foreach (var command in model.Commands)
            {
                EnsureIdentifier(command.Name, $"Command name in model '{model.Name}'");
                ValidateCommandOutcome(model, command, modelNames);
                if (command.ParameterProperty is null)
                    continue;
                if (!properties.TryGetValue(command.ParameterProperty, out var property))
                    throw new InvalidOperationException(
                        $"Command '{model.Name}.{command.Name}' references unknown parameter property '{command.ParameterProperty}'.");
                if (!property.Writable)
                    throw new InvalidOperationException(
                        $"Command parameter property '{model.Name}.{property.Name}' must be writable.");
                if (property.Kind is not (ViewModelValueKind.String or ViewModelValueKind.Integer
                    or ViewModelValueKind.Boolean or ViewModelValueKind.Double))
                    throw new InvalidOperationException(
                        $"Command parameter property '{model.Name}.{property.Name}' must be a string, integer, boolean, or double.");
                if (property.Nullable)
                    throw new InvalidOperationException(
                        $"Command parameter property '{model.Name}.{property.Name}' must not be nullable.");
            }
            ValidateDisplayPath(model, models: Models);
            ValidateThemeVariantProperty(model, properties);
            ValidateRecentFiles(model);
            foreach (var menu in model.Menus)
                ValidateMenu(model, menu, enumsByName);
        }

        ValidateAcyclicModelGraph();
    }

    private static void ValidateTable(
        ViewModelDefinition owner,
        ViewModelCollection collection,
        IReadOnlyDictionary<string, ViewModelProperty> ownerProperties,
        IReadOnlyList<ViewModelDefinition> models)
    {
        var table = collection.Table;
        if (table is null)
            return;
        if (collection.ElementKind != ViewModelValueKind.Model || collection.ElementModelName is null)
            throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' must contain nested view models.");
        if (table.Columns.Count == 0)
            throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' must declare at least one column.");
        EnsureUnique(table.Columns, column => column.Id, $"{owner.Name}.{collection.Name} table column ID");
        EnsureUnique(table.Columns, column => column.Name, $"{owner.Name}.{collection.Name} table column name");
        var row = models.Single(model => model.Name == collection.ElementModelName);
        foreach (var column in table.Columns)
        {
            if (column.Id <= 0)
                throw new InvalidOperationException($"Table column '{owner.Name}.{collection.Name}.{column.Name}' ID must be positive.");
            EnsureIdentifier(column.Name, $"Table column name in '{owner.Name}.{collection.Name}'");
            EnsureNotBlank(column.Header, $"Header for table column '{owner.Name}.{collection.Name}.{column.Name}'");
            ValidateRowPath(owner, collection, row, column.Path, models);
            if (column.Width is { } width && width < 0 ||
                column.MinWidth is { } min && min < 0 ||
                column.MaxWidth is { } max && max < 0 ||
                column.MinWidth is { } minimum && column.MaxWidth is { } maximum && minimum > maximum)
                throw new InvalidOperationException($"Table column '{owner.Name}.{collection.Name}.{column.Name}' has invalid width limits.");
            if (column.Width is null && !column.Star && !column.Auto ||
                column.Width is not null && (column.Star || column.Auto))
                throw new InvalidOperationException($"Table column '{owner.Name}.{collection.Name}.{column.Name}' must declare exactly one width mode.");
        }
        if (table.Selection is { } selection)
        {
            ValidateSelectionProperty(selection.SelectedIndexProperty, ViewModelValueKind.Integer, "selectedIndexProperty");
            if (selection.SelectedKeyProperty is { } keyName)
            {
                if (!ownerProperties.TryGetValue(keyName, out var key) || !key.Writable || key.Nullable ||
                    key.Kind is not (ViewModelValueKind.String or ViewModelValueKind.Integer))
                    throw new InvalidOperationException(
                        $"Table '{owner.Name}.{collection.Name}' selectedKeyProperty '{keyName}' must reference a writable non-nullable String or Integer property.");
            }
            if (selection.SelectedIndexProperty is null && selection.SelectedKeyProperty is null)
                throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' selection must declare an index or key property.");
            if (selection.RowKeyPath is not null)
                ValidateRowPath(owner, collection, row, selection.RowKeyPath, models);
        }
        if (table.Sort is { } sort)
        {
            if (!table.Columns.Any(column => column.Name == sort.Column))
                throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' sort references unknown column '{sort.Column}'.");
            var command = owner.Commands.SingleOrDefault(candidate => candidate.Name == sort.Command)
                ?? throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' sort references unknown command '{sort.Command}'.");
            if (command.ParameterProperty is not null)
                throw new InvalidOperationException($"Table sort command '{owner.Name}.{command.Name}' must not use parameterProperty.");
            if (!ownerProperties.TryGetValue(sort.DirectionProperty, out var direction) ||
                direction.Kind != ViewModelValueKind.String || direction.Nullable)
                throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' directionProperty '{sort.DirectionProperty}' must reference a non-nullable String property.");
        }
        return;

        void ValidateSelectionProperty(string? name, ViewModelValueKind kind, string field)
        {
            if (name is null)
                return;
            if (!ownerProperties.TryGetValue(name, out var property) || !property.Writable ||
                property.Kind != kind || property.Nullable)
                throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' {field} '{name}' must reference a writable non-nullable {kind} property.");
        }
    }

    private static void ValidateWindow(ViewModelDefinition owner, ViewModelCollection collection)
    {
        var window = collection.Window;
        if (window is null)
            return;
        if (window.PageSize <= 0)
            throw new InvalidOperationException($"Windowed collection '{owner.Name}.{collection.Name}' pageSize must be positive.");
        if (window.MaxLivePages <= 0)
            throw new InvalidOperationException($"Windowed collection '{owner.Name}.{collection.Name}' maxLivePages must be positive.");
        // A range batch transports either a nested model or text, so a
        // scalar-number element has no windowed wire form.
        if (collection.ElementKind is ViewModelValueKind.Integer or ViewModelValueKind.Double)
            throw new InvalidOperationException(
                $"Windowed collection '{owner.Name}.{collection.Name}' must contain strings or nested view models.");
        if (collection.Tree is not null)
            throw new InvalidOperationException($"Collection '{owner.Name}.{collection.Name}' cannot be both windowed and a tree root.");
        if (collection.Recursive)
            throw new InvalidOperationException($"Collection '{owner.Name}.{collection.Name}' cannot be both windowed and recursive.");
    }

    private static void ValidateTree(
        ViewModelDefinition owner,
        ViewModelCollection collection,
        IReadOnlyList<ViewModelDefinition> models)
    {
        if (collection.Recursive)
        {
            if (collection.ElementKind != ViewModelValueKind.Model || collection.ElementModelName != owner.Name)
                throw new InvalidOperationException(
                    $"Recursive collection '{owner.Name}.{collection.Name}' must contain elements of its own model '{owner.Name}'.");
        }
        var tree = collection.Tree;
        if (tree is null)
            return;
        if (collection.ElementKind != ViewModelValueKind.Model || collection.ElementModelName is null)
            throw new InvalidOperationException($"Tree '{owner.Name}.{collection.Name}' must contain nested view models.");
        var node = models.Single(model => model.Name == collection.ElementModelName);
        var children = node.Collections.SingleOrDefault(candidate => candidate.Name == tree.ChildrenCollection)
            ?? throw new InvalidOperationException(
                $"Tree '{owner.Name}.{collection.Name}' references unknown children collection '{tree.ChildrenCollection}' on '{node.Name}'.");
        if (children.ElementKind != ViewModelValueKind.Model || children.ElementModelName != node.Name)
            throw new InvalidOperationException(
                $"Tree '{owner.Name}.{collection.Name}' children collection '{node.Name}.{children.Name}' must contain '{node.Name}' elements.");
        if (!children.Recursive)
            throw new InvalidOperationException(
                $"Tree '{owner.Name}.{collection.Name}' children collection '{node.Name}.{children.Name}' must declare 'recursive'.");
        ValidateNodePath(owner, collection, node, tree.HeaderPath, models, ViewModelValueKind.String);
        if (tree.HasChildrenProperty is { } hasChildren)
        {
            var property = node.Properties.SingleOrDefault(candidate => candidate.Name == hasChildren)
                ?? throw new InvalidOperationException(
                    $"Tree '{owner.Name}.{collection.Name}' hasChildrenProperty '{hasChildren}' is not a property of '{node.Name}'.");
            if (property.Kind != ViewModelValueKind.Boolean)
                throw new InvalidOperationException(
                    $"Tree '{owner.Name}.{collection.Name}' hasChildrenProperty '{hasChildren}' must be a Boolean property.");
        }
    }

    private static void ValidateCommandOutcome(
        ViewModelDefinition model,
        ViewModelCommand command,
        IReadOnlySet<string> modelNames)
    {
        if (command.ResultModelName is { } result)
        {
            if (!modelNames.Contains(result))
                throw new InvalidOperationException(
                    $"Command '{model.Name}.{command.Name}' references unknown result model '{result}'.");
        }
        if ((command.SupportsProgress || command.SupportsCancellation) && !command.IsAsync)
            throw new InvalidOperationException(
                $"Command '{model.Name}.{command.Name}' must be asynchronous to support progress or cancellation.");
    }

    private static bool HasRicherDataShapes(ViewModelDefinition model) =>
        model.Maps.Count > 0 ||
        model.Collections.Any(collection => collection.Window is not null || collection.Tree is not null || collection.Recursive) ||
        model.Commands.Any(command => command.ResultModelName is not null || command.SupportsProgress || command.SupportsCancellation);

    private static bool HasCommandSurfaces(ViewModelDefinition model) =>
        model.Menus.Count > 0 ||
        model.RecentFiles is not null ||
        model.DisplayPath is not null;

    /// <summary>
    /// Validates the optional declared display path: a dotted path on the model
    /// itself, ending in a string property, that the generated adapter projects
    /// as <c>ToString()</c>. It is what gives a row, list item or tree node a
    /// meaningful accessible name instead of the adapter's CLR type name.
    /// </summary>
    private static void ValidateDisplayPath(
        ViewModelDefinition model,
        IReadOnlyList<ViewModelDefinition> models)
    {
        if (model.DisplayPath is not { } path)
            return;
        EnsureNotBlank(path, $"Display path for model '{model.Name}'");
        var current = model;
        var segments = path.Split('.');
        for (var index = 0; index < segments.Length; index++)
        {
            var segment = segments[index];
            EnsureIdentifier(segment, $"Display path segment in model '{model.Name}'");
            var property = current.Properties.SingleOrDefault(candidate => candidate.Name == segment)
                ?? throw new InvalidOperationException(
                    $"Model '{model.Name}' displayPath '{path}' references unknown property '{segment}' on '{current.Name}'.");
            if (index == segments.Length - 1)
            {
                if (property.Kind != ViewModelValueKind.String)
                    throw new InvalidOperationException(
                        $"Model '{model.Name}' displayPath '{path}' must end in a String property.");
                return;
            }

            if (property.Kind != ViewModelValueKind.Model || property.ModelName is null)
                throw new InvalidOperationException(
                    $"Model '{model.Name}' displayPath '{path}' cannot traverse scalar property '{segment}'.");
            current = models.Single(candidate => candidate.Name == property.ModelName);
        }
    }

    private static void ValidateThemeVariantProperty(
        ViewModelDefinition model,
        IReadOnlyDictionary<string, ViewModelProperty> properties)
    {
        if (model.ThemeVariantProperty is not { } name)
            return;
        if (!properties.TryGetValue(name, out var property) || !property.Writable || property.Nullable ||
            property.Kind != ViewModelValueKind.Boolean)
            throw new InvalidOperationException(
                $"Model '{model.Name}' themeVariantProperty '{name}' must reference a writable non-nullable Boolean property.");
    }

    /// <summary>
    /// Validates the recent-file list. It deliberately reuses the already
    /// published string-collection transport: an entry <em>is</em> a stage 29
    /// storage URI, which is the only member a storage item is guaranteed to
    /// have, and the menu header is derived from it. No new ABI is introduced.
    /// </summary>
    private static void ValidateRecentFiles(ViewModelDefinition model)
    {
        if (model.RecentFiles is not { } recent)
            return;
        if (recent.Capacity <= 0)
            throw new InvalidOperationException(
                $"Recent files on '{model.Name}' must declare a positive capacity.");
        var collection = model.Collections.SingleOrDefault(candidate => candidate.Name == recent.Collection)
            ?? throw new InvalidOperationException(
                $"Recent files on '{model.Name}' references unknown collection '{recent.Collection}'.");
        if (collection.ElementKind != ViewModelValueKind.String)
            throw new InvalidOperationException(
                $"Recent files collection '{model.Name}.{collection.Name}' must contain strings (storage URIs).");
        if (collection.Window is not null || collection.Table is not null || collection.Tree is not null)
            throw new InvalidOperationException(
                $"Recent files collection '{model.Name}.{collection.Name}' must be a plain string collection.");
        var command = model.Commands.SingleOrDefault(candidate => candidate.Name == recent.ActivateCommand)
            ?? throw new InvalidOperationException(
                $"Recent files on '{model.Name}' references unknown activate command '{recent.ActivateCommand}'.");
        if (command.ParameterProperty is not null)
            throw new InvalidOperationException(
                $"Recent files activate command '{model.Name}.{command.Name}' must not use parameterProperty; the selected URI is passed as the command parameter.");
    }

    private static void ValidateMenu(
        ViewModelDefinition model,
        ViewModelMenu menu,
        IReadOnlyDictionary<string, ViewModelEnumDefinition> enumsByName)
    {
        EnsureIdentifier(menu.Name, $"Menu name in model '{model.Name}'");
        if (menu.Items.Count == 0)
            throw new InvalidOperationException($"Menu '{model.Name}.{menu.Name}' must declare at least one item.");
        if (menu.Kind == ViewModelMenuKind.Accelerators)
        {
            foreach (var item in menu.Items)
            {
                if (item.Kind is not (ViewModelMenuItemKind.Command or ViewModelMenuItemKind.Toggle or ViewModelMenuItemKind.Radio))
                    throw new InvalidOperationException(
                        $"Accelerator menu '{model.Name}.{menu.Name}' item '{item.Name}' must be a command, toggle or radio item.");
                if (item.Gesture is null)
                    throw new InvalidOperationException(
                        $"Accelerator menu '{model.Name}.{menu.Name}' item '{item.Name}' must declare a gesture.");
            }
        }
        var ids = new HashSet<int>();
        var names = new HashSet<string>(StringComparer.Ordinal);
        ValidateMenuItems(model, menu, menu.Items, ids, names, enumsByName, depth: 0);
    }

    /// <summary>
    /// The key menu item names are made unique by.
    /// </summary>
    /// <remarks>
    /// Every item of a menu, nested submenu items included, is projected into
    /// one generated method body whose local names are the item name with a
    /// lower-cased first character. Two names that differ only in that first
    /// character would therefore collide as generated locals, so uniqueness is
    /// enforced on the same key rather than on the verbatim name - a schema
    /// error is a far better report than a compile error in generated code.
    /// </remarks>
    private static string MenuItemNameKey(string name) =>
        char.ToLowerInvariant(name[0]) + name[1..];

    private static void ValidateMenuItems(
        ViewModelDefinition model,
        ViewModelMenu menu,
        IReadOnlyList<ViewModelMenuItem> items,
        HashSet<int> ids,
        HashSet<string> names,
        IReadOnlyDictionary<string, ViewModelEnumDefinition> enumsByName,
        int depth)
    {
        if (depth > MaxMenuDepth)
            throw new InvalidOperationException(
                $"Menu '{model.Name}.{menu.Name}' nests deeper than {MaxMenuDepth} levels.");
        foreach (var item in items)
        {
            if (item.Id <= 0)
                throw new InvalidOperationException($"Menu item ID '{item.Id}' in '{model.Name}.{menu.Name}' must be positive.");
            if (!ids.Add(item.Id))
                throw new InvalidOperationException($"Duplicate menu item ID '{item.Id}' in '{model.Name}.{menu.Name}'.");
            EnsureIdentifier(item.Name, $"Menu item name in '{model.Name}.{menu.Name}'");
            if (!names.Add(MenuItemNameKey(item.Name)))
                throw new InvalidOperationException($"Duplicate menu item name '{item.Name}' in '{model.Name}.{menu.Name}'.");
            ValidateMenuItem(model, menu, item, enumsByName);
            if (item.Items.Count > 0)
                ValidateMenuItems(model, menu, item.Items, ids, names, enumsByName, depth + 1);
        }
    }

    private static void ValidateMenuItem(
        ViewModelDefinition model,
        ViewModelMenu menu,
        ViewModelMenuItem item,
        IReadOnlyDictionary<string, ViewModelEnumDefinition> enumsByName)
    {
        var where = $"Menu item '{model.Name}.{menu.Name}.{item.Name}'";
        if (item.Kind == ViewModelMenuItemKind.Separator)
        {
            if (item.Header is not null || item.Command is not null || item.Gesture is not null ||
                item.ToggleProperty is not null || item.RadioProperty is not null || item.Items.Count > 0)
                throw new InvalidOperationException($"{where} is a separator and cannot declare any other member.");
            return;
        }

        EnsureNotBlank(item.Header ?? "", $"Header for {where.ToLowerInvariant()}");
        if (item.Gesture is { } gesture)
            EnsureNotBlank(gesture, $"Gesture for {where.ToLowerInvariant()}");
        if (item.IsEnabledProperty is { } enabled)
            RequireBooleanProperty(model, enabled, $"{where} isEnabledProperty");

        switch (item.Kind)
        {
            case ViewModelMenuItemKind.Command:
                if (item.Command is null)
                    throw new InvalidOperationException($"{where} must declare a command.");
                RequireCommand(model, item.Command, where);
                RejectSubItems(item, where);
                break;
            case ViewModelMenuItemKind.Submenu:
                if (item.Items.Count == 0)
                    throw new InvalidOperationException($"{where} is a submenu and must declare items.");
                if (item.Command is not null || item.Gesture is not null)
                    throw new InvalidOperationException($"{where} is a submenu and cannot declare a command or gesture.");
                break;
            case ViewModelMenuItemKind.Toggle:
                if (item.ToggleProperty is null)
                    throw new InvalidOperationException($"{where} must declare a toggleProperty.");
                var toggle = RequireBooleanProperty(model, item.ToggleProperty, $"{where} toggleProperty");
                if (!toggle.Writable)
                    throw new InvalidOperationException($"{where} toggleProperty '{toggle.Name}' must be writable.");
                if (item.Command is not null)
                    RequireCommand(model, item.Command, where);
                RejectSubItems(item, where);
                break;
            case ViewModelMenuItemKind.Radio:
                if (item.RadioProperty is null || item.RadioValue is null)
                    throw new InvalidOperationException($"{where} must declare a radioProperty and a radioValue.");
                ValidateRadio(model, item, enumsByName, where);
                if (item.Command is not null)
                    RequireCommand(model, item.Command, where);
                RejectSubItems(item, where);
                break;
            case ViewModelMenuItemKind.RecentFiles:
                if (model.RecentFiles is null)
                    throw new InvalidOperationException($"{where} is a recent-file submenu but '{model.Name}' declares no recentFiles.");
                if (item.Command is not null || item.Gesture is not null)
                    throw new InvalidOperationException($"{where} is a recent-file submenu and cannot declare a command or gesture.");
                RejectSubItems(item, where);
                break;
            default:
                throw new InvalidOperationException($"{where} has an unsupported kind '{item.Kind}'.");
        }

        if (item.ToggleProperty is not null && item.Kind != ViewModelMenuItemKind.Toggle)
            throw new InvalidOperationException($"{where} declares a toggleProperty but is not a toggle.");
        if ((item.RadioProperty is not null || item.RadioValue is not null) && item.Kind != ViewModelMenuItemKind.Radio)
            throw new InvalidOperationException($"{where} declares radio members but is not a radio item.");

        static void RejectSubItems(ViewModelMenuItem item, string where)
        {
            if (item.Items.Count > 0)
                throw new InvalidOperationException($"{where} cannot declare nested items.");
        }
    }

    private static void ValidateRadio(
        ViewModelDefinition model,
        ViewModelMenuItem item,
        IReadOnlyDictionary<string, ViewModelEnumDefinition> enumsByName,
        string where)
    {
        var property = model.Properties.SingleOrDefault(candidate => candidate.Name == item.RadioProperty)
            ?? throw new InvalidOperationException($"{where} radioProperty '{item.RadioProperty}' is not a property of '{model.Name}'.");
        if (!property.Writable)
            throw new InvalidOperationException($"{where} radioProperty '{property.Name}' must be writable.");
        if (property.Nullable)
            throw new InvalidOperationException($"{where} radioProperty '{property.Name}' must not be nullable.");
        switch (property.Kind)
        {
            case ViewModelValueKind.String:
                break;
            case ViewModelValueKind.Enum:
                var enumDefinition = enumsByName[property.EnumName!];
                if (!enumDefinition.Members.Any(member => member.Name == item.RadioValue))
                    throw new InvalidOperationException(
                        $"{where} radioValue '{item.RadioValue}' is not a member of enum '{enumDefinition.Name}'.");
                break;
            default:
                throw new InvalidOperationException(
                    $"{where} radioProperty '{property.Name}' must be a String or Enum property.");
        }
    }

    private static ViewModelProperty RequireBooleanProperty(
        ViewModelDefinition model,
        string name,
        string where)
    {
        var property = model.Properties.SingleOrDefault(candidate => candidate.Name == name)
            ?? throw new InvalidOperationException($"{where} '{name}' is not a property of '{model.Name}'.");
        if (property.Kind != ViewModelValueKind.Boolean)
            throw new InvalidOperationException($"{where} '{name}' must be a Boolean property.");
        return property;
    }

    private static void RequireCommand(ViewModelDefinition model, string name, string where)
    {
        if (model.Commands.All(command => command.Name != name))
            throw new InvalidOperationException($"{where} references unknown command '{name}'.");
    }

    /// <summary>Maximum nesting depth of a declared menu, submenus included.</summary>
    public const int MaxMenuDepth = 8;

    private static void ValidateNodePath(
        ViewModelDefinition owner, ViewModelCollection collection, ViewModelDefinition node,
        string path, IReadOnlyList<ViewModelDefinition> models, ViewModelValueKind terminalKind)
    {
        EnsureNotBlank(path, $"Node path in tree '{owner.Name}.{collection.Name}'");
        var current = node;
        var segments = path.Split('.');
        for (var index = 0; index < segments.Length; index++)
        {
            var segment = segments[index];
            EnsureIdentifier(segment, $"Path segment '{segment}' in tree '{owner.Name}.{collection.Name}'");
            var property = current.Properties.SingleOrDefault(candidate => candidate.Name == segment)
                ?? throw new InvalidOperationException($"Tree '{owner.Name}.{collection.Name}' path '{path}' references unknown property '{segment}' on '{current.Name}'.");
            if (index == segments.Length - 1)
            {
                if (property.Kind != terminalKind)
                    throw new InvalidOperationException($"Tree '{owner.Name}.{collection.Name}' path '{path}' must end in a {terminalKind} property.");
                return;
            }
            if (property.Kind != ViewModelValueKind.Model || property.ModelName is null)
                throw new InvalidOperationException($"Tree '{owner.Name}.{collection.Name}' path '{path}' cannot traverse scalar property '{segment}'.");
            current = models.Single(model => model.Name == property.ModelName);
        }
    }

    private static void ValidateRowPath(
        ViewModelDefinition owner, ViewModelCollection collection, ViewModelDefinition row,
        string path, IReadOnlyList<ViewModelDefinition> models)
    {
        EnsureNotBlank(path, $"Row path in table '{owner.Name}.{collection.Name}'");
        var current = row;
        var segments = path.Split('.');
        for (var index = 0; index < segments.Length; index++)
        {
            var segment = segments[index];
            EnsureIdentifier(segment, $"Path segment '{segment}' in table '{owner.Name}.{collection.Name}'");
            var property = current.Properties.SingleOrDefault(candidate => candidate.Name == segment)
                ?? throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' path '{path}' references unknown property '{segment}' on '{current.Name}'.");
            if (index == segments.Length - 1)
            {
                if (property.Kind == ViewModelValueKind.Model)
                    throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' path '{path}' must end in a scalar property.");
                return;
            }
            if (property.Kind != ViewModelValueKind.Model || property.ModelName is null)
                throw new InvalidOperationException($"Table '{owner.Name}.{collection.Name}' path '{path}' cannot traverse scalar property '{segment}'.");
            current = models.Single(model => model.Name == property.ModelName);
        }
    }

    /// <summary>
    /// Rejects an accidental recursive schema graph. A collection explicitly
    /// declared <c>recursive</c> is the one deliberate exception: it is how a
    /// hierarchical (tree) model is expressed, and its self-edge is skipped
    /// here because the generated adapters own children lazily, per node,
    /// exactly like any other nested collection.
    /// </summary>
    private void ValidateAcyclicModelGraph()
    {
        var edges = Models.ToDictionary(
            model => model.Name,
            model => model.Properties
                .Where(property => property.Kind == ViewModelValueKind.Model)
                .Select(property => property.ModelName!)
                .Concat(model.Collections
                    .Where(collection => collection.ElementKind == ViewModelValueKind.Model && !collection.Recursive)
                    .Select(collection => collection.ElementModelName!))
                .Concat(model.Maps
                    .Where(map => map.ValueKind == ViewModelValueKind.Model)
                    .Select(map => map.ValueModelName!))
                .ToArray(),
            StringComparer.Ordinal);
        var states = new Dictionary<string, VisitState>(StringComparer.Ordinal);
        var path = new List<string>();

        foreach (var model in Models)
            Visit(model.Name);

        void Visit(string model)
        {
            if (states.TryGetValue(model, out var state))
            {
                if (state == VisitState.Visiting)
                {
                    var start = path.IndexOf(model);
                    var cycle = path.Skip(start).Append(model);
                    throw new InvalidOperationException(
                        $"Recursive view-model schema graph is not supported: {string.Join(" -> ", cycle)}.");
                }

                return;
            }

            states.Add(model, VisitState.Visiting);
            path.Add(model);
            foreach (var child in edges[model])
                Visit(child);
            path.RemoveAt(path.Count - 1);
            states[model] = VisitState.Visited;
        }
    }

    private enum VisitState
    {
        Visiting,
        Visited,
    }

    private static void ValidateInitialValue(
        ViewModelDefinition model,
        ViewModelProperty property)
    {
        var invalid = property.Kind switch
        {
            ViewModelValueKind.String =>
                property.InitialInteger is not null ||
                property.InitialBoolean is not null ||
                property.InitialDouble is not null,
            ViewModelValueKind.Integer or ViewModelValueKind.Enum =>
                property.InitialString is not null ||
                property.InitialBoolean is not null ||
                property.InitialDouble is not null,
            ViewModelValueKind.Boolean =>
                property.InitialString is not null ||
                property.InitialInteger is not null ||
                property.InitialDouble is not null,
            ViewModelValueKind.Double =>
                property.InitialString is not null ||
                property.InitialInteger is not null ||
                property.InitialBoolean is not null,
            ViewModelValueKind.Model =>
                property.InitialString is not null ||
                property.InitialInteger is not null ||
                property.InitialBoolean is not null ||
                property.InitialDouble is not null,
            _ => true,
        };
        if (invalid)
            throw new InvalidOperationException(
                $"Property '{model.Name}.{property.Name}' has an initializer for a different value kind.");
    }

    private static void ValidateEnumProperty(
        ViewModelDefinition model,
        ViewModelProperty property,
        IReadOnlyDictionary<string, ViewModelEnumDefinition> enumsByName)
    {
        if (property.Kind == ViewModelValueKind.Enum)
        {
            if (property.EnumName is null)
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' must declare 'enumName'.");
            if (!enumsByName.TryGetValue(property.EnumName, out var enumDefinition))
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' references unknown enum '{property.EnumName}'.");
            if (property.InitialInteger is { } initial &&
                !enumDefinition.Members.Any(member => member.Value == initial))
            {
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' has an initial value that is not a member of enum '{property.EnumName}'.");
            }
            if (property.Nullable)
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' is an enum property and cannot be nullable in this stage.");
        }
        else if (property.EnumName is not null)
        {
            throw new InvalidOperationException(
                $"Property '{model.Name}.{property.Name}' declares 'enumName' but is not an enum property.");
        }
    }

    private static void ValidateModelProperty(
        ViewModelDefinition model,
        ViewModelProperty property,
        IReadOnlySet<string> modelNames)
    {
        if (property.Kind == ViewModelValueKind.Model)
        {
            if (property.ModelName is null)
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' must declare 'modelName'.");
            if (!modelNames.Contains(property.ModelName))
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' references unknown model '{property.ModelName}'.");
            if (property.Writable)
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' is a nested view model and cannot be writable from managed code.");
            if (!property.Nullable)
                throw new InvalidOperationException(
                    $"Property '{model.Name}.{property.Name}' is a nested view model and must be nullable.");
        }
        else if (property.ModelName is not null)
        {
            throw new InvalidOperationException(
                $"Property '{model.Name}.{property.Name}' declares 'modelName' but is not a nested view model.");
        }
        else if (property.Kind != ViewModelValueKind.String && property.Nullable)
        {
            throw new InvalidOperationException(
                $"Property '{model.Name}.{property.Name}' is nullable but only string and nested view-model properties support nullability in this stage.");
        }
    }

    private static bool IsScalarKind(ViewModelValueKind kind) =>
        kind is ViewModelValueKind.String or ViewModelValueKind.Integer or ViewModelValueKind.Boolean or ViewModelValueKind.Double;

    private static void EnsureNotBlank(string value, string description)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new InvalidOperationException($"{description} cannot be empty.");
    }

    private static void EnsureIdentifier(string value, string description)
    {
        EnsureNotBlank(value, description);
        if (!(char.IsLetter(value[0]) || value[0] == '_') || value.Skip(1).Any(character => !char.IsLetterOrDigit(character) && character != '_'))
            throw new InvalidOperationException($"{description} '{value}' is not a valid identifier.");
    }

    private static void EnsurePositiveIds<T>(
        IEnumerable<T> values,
        Func<T, int> id,
        string description)
    {
        foreach (var value in values)
        {
            if (id(value) <= 0)
                throw new InvalidOperationException(
                    $"{description} ID '{id(value)}' must be positive.");
        }
    }

    private static void EnsureUnique<T, TKey>(
        IEnumerable<T> values,
        Func<T, TKey> key,
        string description)
        where TKey : notnull
    {
        var seen = new HashSet<TKey>();
        foreach (var value in values)
        {
            if (!seen.Add(key(value)))
                throw new InvalidOperationException($"Duplicate {description} '{key(value)}'.");
        }
    }

    private static JsonSerializerOptions JsonOptions { get; } = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Converters = { new JsonStringEnumConverter() },
    };
}

public sealed class ViewModelDefinition
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required string ManagedNamespace { get; init; }
    public IReadOnlyList<ViewModelProperty> Properties { get; init; } = [];
    public IReadOnlyList<ViewModelCollection> Collections { get; init; } = [];

    /// <summary>
    /// Observable keyed maps owned by this model. Maps have their own ID space
    /// (like properties, collections and commands) because they are addressed
    /// by dedicated map transport operations.
    /// </summary>
    public IReadOnlyList<ViewModelMap> Maps { get; init; } = [];

    public IReadOnlyList<ViewModelCommand> Commands { get; init; } = [];

    /// <summary>
    /// Application, context and accelerator-only menus bound to this model's
    /// generated commands and properties. Menus are presentation, so nothing
    /// here crosses the ABI: the generated managed factory wires already
    /// generated commands and property notifications.
    /// </summary>
    public IReadOnlyList<ViewModelMenu> Menus { get; init; } = [];

    /// <summary>Optional generated recent-file list over a declared string (URI) collection.</summary>
    public ViewModelRecentFiles? RecentFiles { get; init; }

    /// <summary>
    /// Optional dotted path on this model, ending in a string property, that
    /// the generated adapter projects as <c>ToString()</c>. Presentation and UI
    /// automation fall back to <c>ToString()</c> for a data item, so declaring
    /// this is what stops a row reading as its CLR adapter type name.
    /// </summary>
    public string? DisplayPath { get; init; }

    /// <summary>
    /// Optional writable Boolean property that drives
    /// <c>Application.RequestedThemeVariant</c> (true = Dark, false = Light)
    /// from the generated adapter, with no window code-behind.
    /// </summary>
    public string? ThemeVariantProperty { get; init; }
}

/// <summary>What a declared menu is projected as.</summary>
public enum ViewModelMenuKind
{
    /// <summary>
    /// The application/window menu. Projected as an Avalonia <c>NativeMenu</c>
    /// set on a top-level, which the platform exports natively where it has a
    /// menu bar (macOS) and which <c>NativeMenuBar</c> renders in-window
    /// elsewhere. Declared gestures additionally become window key bindings,
    /// because only a native menu bar handles its own shortcuts.
    /// </summary>
    Application,

    /// <summary>
    /// A context menu. Projected as a generated <c>ContextMenu</c> subclass so
    /// compiled AXAML can attach it to any control and inherit its data context.
    /// </summary>
    Context,

    /// <summary>
    /// Keyboard accelerators with no visual menu. Projected only as key
    /// bindings, for shortcuts that are not on any menu item.
    /// </summary>
    Accelerators,
}

/// <summary>What one declared menu item does.</summary>
public enum ViewModelMenuItemKind
{
    /// <summary>Invokes a generated command.</summary>
    Command,

    /// <summary>A separator line. Carries no other member.</summary>
    Separator,

    /// <summary>A checkable item mirroring (and writing) a writable Boolean property.</summary>
    Toggle,

    /// <summary>One option of a radio group over a writable String or Enum property.</summary>
    Radio,

    /// <summary>A nested submenu of further items.</summary>
    Submenu,

    /// <summary>A submenu populated from the model's declared recent-file list.</summary>
    RecentFiles,
}

/// <summary>One declared menu owned by a view model.</summary>
public sealed class ViewModelMenu
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required ViewModelMenuKind Kind { get; init; }
    public IReadOnlyList<ViewModelMenuItem> Items { get; init; } = [];
}

/// <summary>One item in a declared menu.</summary>
public sealed class ViewModelMenuItem
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required ViewModelMenuItemKind Kind { get; init; }

    /// <summary>Display text. Required for every kind except <see cref="ViewModelMenuItemKind.Separator"/>.</summary>
    public string? Header { get; init; }

    /// <summary>Name of the command (without the generated <c>Command</c> suffix) this item invokes.</summary>
    public string? Command { get; init; }

    /// <summary>
    /// Keyboard accelerator in Avalonia <see cref="Avalonia.Input.KeyGesture"/>
    /// syntax, for example <c>Ctrl+O</c> or <c>Ctrl+Shift+S</c>. The literal
    /// text is carried through to <c>KeyGesture.Parse</c>; the platform-specific
    /// <c>Cmd</c> mapping stays Avalonia's concern.
    /// </summary>
    public string? Gesture { get; init; }

    /// <summary>Writable Boolean property mirrored by, and written from, a toggle item.</summary>
    public string? ToggleProperty { get; init; }

    /// <summary>Writable String or Enum property a radio item writes.</summary>
    public string? RadioProperty { get; init; }

    /// <summary>Value written to <see cref="RadioProperty"/>: an enum member name, or a literal string.</summary>
    public string? RadioValue { get; init; }

    /// <summary>Optional Boolean property controlling this item's enabled state.</summary>
    public string? IsEnabledProperty { get; init; }

    /// <summary>Child items of a <see cref="ViewModelMenuItemKind.Submenu"/>.</summary>
    public IReadOnlyList<ViewModelMenuItem> Items { get; init; } = [];
}

/// <summary>
/// A generated most-recently-used file list.
/// </summary>
/// <remarks>
/// The list is deliberately a plain string collection of stage 29 storage URIs:
/// a URI is the only member <c>IStorageItem</c> guarantees, so it is the stable
/// identity, and the menu header is derived from it. That keeps recent files on
/// the already published collection transport with no new ABI and nothing
/// platform-specific (this is not a Windows jump list).
/// </remarks>
public sealed class ViewModelRecentFiles
{
    public required int Id { get; init; }

    /// <summary>Name of the string collection holding the URIs, most recent first.</summary>
    public required string Collection { get; init; }

    /// <summary>Command invoked with the chosen URI as its command parameter.</summary>
    public required string ActivateCommand { get; init; }

    /// <summary>Maximum retained entries. Enforced by the generated Rust list.</summary>
    public int Capacity { get; init; } = 10;
}

/// <summary>
/// A schema-declared observable keyed map. The key kind is fixed
/// (<see cref="ViewModelValueKind.String"/> or
/// <see cref="ViewModelValueKind.Integer"/>) so the transport never has to
/// guess how to decode a key, and the value kind may be a scalar or a nested
/// model whose ownership follows exactly the same rules as a nested
/// collection element.
/// </summary>
public sealed class ViewModelMap
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required ViewModelValueKind KeyKind { get; init; }
    public required ViewModelValueKind ValueKind { get; init; }

    /// <summary>Name of the nested model each value is. Required when <see cref="ValueKind"/> is <see cref="ViewModelValueKind.Model"/>.</summary>
    public string? ValueModelName { get; init; }
}

public sealed class ViewModelProperty
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required ViewModelValueKind Kind { get; init; }
    public bool Writable { get; init; }

    /// <summary>
    /// Whether the property may currently hold no value. Only string and
    /// nested view-model (<see cref="ViewModelValueKind.Model"/>) properties
    /// support nullability in this stage; a <see cref="ViewModelValueKind.Model"/>
    /// property is always nullable (there may be no nested view model yet).
    /// A nullable property is published outbound-only through a dedicated
    /// "became null" transport signal (see <c>IAvnRustVmSink2.SetNull</c>);
    /// inbound writes always carry a concrete value.
    /// </summary>
    public bool Nullable { get; init; }

    /// <summary>Name of the enum (from <see cref="ViewModelIr.Enums"/>) this property's values come from. Required when <see cref="Kind"/> is <see cref="ViewModelValueKind.Enum"/>.</summary>
    public string? EnumName { get; init; }

    /// <summary>Name of the nested model (from <see cref="ViewModelIr.Models"/>) this property refers to. Required when <see cref="Kind"/> is <see cref="ViewModelValueKind.Model"/>.</summary>
    public string? ModelName { get; init; }

    public string? InitialString { get; init; }
    public long? InitialInteger { get; init; }
    public bool? InitialBoolean { get; init; }
    public double? InitialDouble { get; init; }
}

public sealed class ViewModelCollection
{
    public required int Id { get; init; }
    public required string Name { get; init; }

    /// <summary>
    /// What each element is. <see cref="ViewModelValueKind.String"/>,
    /// <see cref="ViewModelValueKind.Integer"/> and
    /// <see cref="ViewModelValueKind.Double"/> project a scalar list;
    /// <see cref="ViewModelValueKind.Model"/> projects nested view models and
    /// is the only kind a table, tree or window may use.
    /// </summary>
    public required ViewModelValueKind ElementKind { get; init; }

    /// <summary>Name of the nested model each element is. Required when <see cref="ElementKind"/> is <see cref="ViewModelValueKind.Model"/>, and rejected otherwise.</summary>
    public string? ElementModelName { get; init; }

    /// <summary>Optional first-class presentation metadata for a virtualized table over this collection.</summary>
    public ViewModelTable? Table { get; init; }

    /// <summary>
    /// Optional windowed (range-backed) projection. When present the managed
    /// side never materializes an adapter per element: it exposes the Rust
    /// total count and realizes only the pages the presentation actually asks
    /// for, evicting the least recently used page beyond
    /// <see cref="ViewModelCollectionWindow.MaxLivePages"/>.
    /// </summary>
    public ViewModelCollectionWindow? Window { get; init; }

    /// <summary>
    /// Declares that this collection's elements are of its own owner model,
    /// which is how a hierarchical (tree) node's children are expressed. It is
    /// the only place the schema graph is allowed to be cyclic.
    /// </summary>
    public bool Recursive { get; init; }

    /// <summary>Optional hierarchical metadata making this collection a tree root.</summary>
    public ViewModelTree? Tree { get; init; }
}

/// <summary>Range-backed projection parameters for one collection.</summary>
public sealed class ViewModelCollectionWindow
{
    /// <summary>Number of elements Rust delivers per realized page.</summary>
    public int PageSize { get; init; } = 128;

    /// <summary>Maximum number of pages kept live; older pages are detached and released.</summary>
    public int MaxLivePages { get; init; } = 8;
}

/// <summary>
/// Hierarchical metadata for a collection of nested models, sufficient to
/// author an Avalonia <c>TreeDataTemplate</c> in compiled AXAML. There is no
/// WPF <c>HierarchicalDataTemplate</c> equivalent here: the descriptor names
/// the node's own children collection, its header path, and an optional
/// "has children" flag used for lazy expansion.
/// </summary>
public sealed class ViewModelTree
{
    /// <summary>Name of the recursive children collection on the element model.</summary>
    public required string ChildrenCollection { get; init; }

    /// <summary>Dotted path on the element model that produces the node header text.</summary>
    public required string HeaderPath { get; init; }

    /// <summary>Optional Boolean property on the element model reporting whether children exist but are not loaded.</summary>
    public string? HasChildrenProperty { get; init; }
}

public sealed class ViewModelTable
{
    public IReadOnlyList<ViewModelTableColumn> Columns { get; init; } = [];
    public ViewModelTableSelection? Selection { get; init; }
    public ViewModelTableSort? Sort { get; init; }
}

public sealed class ViewModelTableColumn
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required string Header { get; init; }
    public required string Path { get; init; }
    public double? Width { get; init; }
    public bool Star { get; init; }
    public bool Auto { get; init; }
    public double? MinWidth { get; init; }
    public double? MaxWidth { get; init; }
    public bool Resizable { get; init; } = true;
    public bool Sortable { get; init; }
    public ViewModelTableHorizontalAlignment HorizontalAlignment { get; init; } = ViewModelTableHorizontalAlignment.Left;
}

public enum ViewModelTableHorizontalAlignment { Left, Center, Right, Stretch }

public sealed class ViewModelTableSelection
{
    public string? SelectedIndexProperty { get; init; }
    public string? SelectedKeyProperty { get; init; }
    public string? RowKeyPath { get; init; }
}

public sealed class ViewModelTableSort
{
    public required string Command { get; init; }
    public required string Column { get; init; }
    public required string DirectionProperty { get; init; }
}

public sealed class ViewModelCommand
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public bool IsAsync { get; init; }
    public string? ParameterProperty { get; init; }

    /// <summary>
    /// Optional typed outcome model. When present, the generated adapter
    /// exposes a nullable <c>{Name}Result</c> nested adapter that Rust
    /// publishes with the same nested ownership rules as any model property.
    /// </summary>
    public string? ResultModelName { get; init; }

    /// <summary>Whether Rust publishes determinate/indeterminate progress for this async command.</summary>
    public bool SupportsProgress { get; init; }

    /// <summary>Whether the generated adapter exposes a <c>Cancel{Name}Command</c> for this async command.</summary>
    public bool SupportsCancellation { get; init; }
}

public sealed class ViewDefinition
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required string Model { get; init; }
    public required string ManagedTypeName { get; init; }
    public bool DynamicBindings { get; init; }

    /// <summary>
    /// Names of converters (from <see cref="ViewModelIr.Converters"/>) this
    /// view uses. Purely documentation for the generated contract report and
    /// the generated "with converters" mount API name; the generated
    /// converters trait always contains every schema-wide converter.
    /// </summary>
    public IReadOnlyList<string> Converters { get; init; } = [];
}

/// <summary>
/// A Rust-authored, pure <c>IValueConverter</c>. Converters are schema-wide
/// (not owned by a single model) because they must not read or lock any
/// view-model state; see <see cref="ViewDefinition.Converters"/> for which
/// views are documented to use a given converter.
/// </summary>
public sealed class ValueConverterDefinition
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required string ManagedNamespace { get; init; }
    public required ViewModelValueKind ValueKind { get; init; }
    public ViewModelValueKind? ParameterKind { get; init; }
    public required ViewModelValueKind ResultKind { get; init; }
    public bool SupportsConvertBack { get; init; }
}

public enum ViewModelValueKind
{
    String,
    Integer,
    Boolean,
    Double,

    /// <summary>
    /// An integer-backed named constant set (see <see cref="ViewModelIr.Enums"/>).
    /// Transported on the wire as <see cref="Integer"/> (no new ABI kind);
    /// only the generated C#/Rust surface types it as a real enum.
    /// </summary>
    Enum,

    /// <summary>
    /// A reference to another model in <see cref="ViewModelIr.Models"/>,
    /// owned and republished wholesale by Rust. Always nullable and never
    /// writable from managed code (its own properties/commands are writable
    /// independently, through its own generated adapter).
    /// </summary>
    Model,
}

/// <summary>
/// An integer-backed named constant set shared by both the generated C# enum
/// and the generated Rust <c>#[repr(i64)]</c> enum. Member values are
/// explicit and stable, exactly like property/command/model IDs, so
/// reordering members in the schema cannot change the wire representation.
/// </summary>
public sealed class ViewModelEnumDefinition
{
    public required int Id { get; init; }
    public required string Name { get; init; }
    public required string ManagedNamespace { get; init; }
    public IReadOnlyList<ViewModelEnumMember> Members { get; init; } = [];
}

public sealed class ViewModelEnumMember
{
    public required string Name { get; init; }
    public required long Value { get; init; }
}
