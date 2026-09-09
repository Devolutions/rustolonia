using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using System.Windows.Input;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Rust;

public enum RustViewModelValueKind
{
    String,
    Integer,
    Boolean,
    Double,

    /// <summary>
    /// An integer-backed named constant. Transported on the wire as
    /// <see cref="Integer"/>; the reflectable adapter boxes the concrete
    /// generated enum type (from <see cref="RustViewModelPropertyDescriptor.InitialValue"/>)
    /// so bindings and converters observe a real enum value, not a raw
    /// integer.
    /// </summary>
    Enum,

    /// <summary>
    /// A reference to another view model, exposed as a nested
    /// <see cref="ReflectableRustViewModelAdapter"/> built from
    /// <see cref="RustViewModelPropertyDescriptor.NestedDescriptor"/> (for a
    /// property) or <see cref="RustViewModelCollectionDescriptor.ElementDescriptor"/>
    /// (for a collection element). Always nullable for properties; never
    /// writable from managed code.
    /// </summary>
    Model,
}

public sealed class RustViewModelPropertyDescriptor(
    int id,
    string name,
    RustViewModelValueKind kind,
    bool writable,
    bool nullable,
    object? initialValue,
    RustViewModelDescriptor? nestedDescriptor = null)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public RustViewModelValueKind Kind { get; } = kind;
    public bool Writable { get; } = writable;
    public bool Nullable { get; } = nullable;
    public object? InitialValue { get; } = initialValue;
    public RustViewModelDescriptor? NestedDescriptor { get; } = nestedDescriptor;
}

public sealed class RustViewModelCollectionDescriptor(
    int id,
    string name,
    RustViewModelValueKind elementKind,
    RustViewModelDescriptor? elementDescriptor = null,
    RustTableDescriptor? table = null,
    RustWindowDescriptor? window = null,
    RustTreeDescriptor? tree = null,
    bool recursive = false)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public RustViewModelValueKind ElementKind { get; } = elementKind;
    public RustViewModelDescriptor? ElementDescriptor { get; } = elementDescriptor;
    public RustTableDescriptor? Table { get; } = table;

    /// <summary>Range-backed projection parameters, or null for a fully materialized collection.</summary>
    public RustWindowDescriptor? Window { get; } = window;

    /// <summary>Hierarchical metadata when this collection is a tree root.</summary>
    public RustTreeDescriptor? Tree { get; } = tree;

    /// <summary>True when this collection holds children of its own owner model.</summary>
    public bool Recursive { get; } = recursive;

    public RustViewModelCollectionDescriptor(
        int id,
        string name,
        RustViewModelValueKind elementKind,
        RustViewModelDescriptor? elementDescriptor)
        : this(id, name, elementKind, elementDescriptor, null)
    {
    }
}

/// <summary>Build-time parameters of a range-backed collection projection.</summary>
public sealed class RustWindowDescriptor(int pageSize, int maxLivePages)
{
    public int PageSize { get; } = pageSize;
    public int MaxLivePages { get; } = maxLivePages;
}

/// <summary>
/// Build-time hierarchical metadata. Generated presentation assemblies use it
/// to author an Avalonia <c>TreeDataTemplate</c> in compiled AXAML; no
/// reflection binding is created from it.
/// </summary>
public sealed class RustTreeDescriptor(string childrenCollection, string headerPath, string? hasChildrenProperty)
{
    public string ChildrenCollection { get; } = childrenCollection;
    public string HeaderPath { get; } = headerPath;
    public string? HasChildrenProperty { get; } = hasChildrenProperty;
}

/// <summary>Build-time description of one observable keyed map.</summary>
public sealed class RustViewModelMapDescriptor(
    int id,
    string name,
    RustViewModelValueKind keyKind,
    RustViewModelValueKind valueKind,
    RustViewModelDescriptor? valueDescriptor = null)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public RustViewModelValueKind KeyKind { get; } = keyKind;
    public RustViewModelValueKind ValueKind { get; } = valueKind;
    public RustViewModelDescriptor? ValueDescriptor { get; } = valueDescriptor;
}

/// <summary>
/// Build-time table presentation metadata. Generated presentation assemblies use
/// path data in compiled AXAML; no reflection binding is created from it.
/// </summary>
public sealed class RustTableDescriptor(
    IReadOnlyList<RustTableColumnDescriptor> columns,
    RustTableSelectionDescriptor? selection = null,
    RustTableSortDescriptor? sort = null)
{
    public IReadOnlyList<RustTableColumnDescriptor> Columns { get; } = columns;
    public RustTableSelectionDescriptor? Selection { get; } = selection;
    public RustTableSortDescriptor? Sort { get; } = sort;
}

public sealed class RustTableColumnDescriptor(
    int id, string name, string header, string path, double? width, bool star, bool auto,
    double? minWidth, double? maxWidth, bool resizable, bool sortable,
    RustTableHorizontalAlignment horizontalAlignment)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public string Header { get; } = header;
    public string Path { get; } = path;
    public double? Width { get; } = width;
    public bool Star { get; } = star;
    public bool Auto { get; } = auto;
    public double? MinWidth { get; } = minWidth;
    public double? MaxWidth { get; } = maxWidth;
    public bool Resizable { get; } = resizable;
    public bool Sortable { get; } = sortable;
    public RustTableHorizontalAlignment HorizontalAlignment { get; } = horizontalAlignment;
}

public enum RustTableHorizontalAlignment { Left, Center, Right, Stretch }

public sealed class RustTableSelectionDescriptor(string? selectedIndexProperty, string? selectedKeyProperty, string? rowKeyPath)
{
    public string? SelectedIndexProperty { get; } = selectedIndexProperty;
    public string? SelectedKeyProperty { get; } = selectedKeyProperty;
    public string? RowKeyPath { get; } = rowKeyPath;
}

public sealed class RustTableSortDescriptor(string command, string column, string directionProperty)
{
    public string Command { get; } = command;
    public string Column { get; } = column;
    public string DirectionProperty { get; } = directionProperty;
}

public sealed class RustViewModelCommandDescriptor(
    int id,
    string name,
    bool isAsync,
    string? parameterProperty,
    bool acceptsParameter = false,
    RustViewModelDescriptor? resultDescriptor = null,
    bool supportsProgress = false,
    bool supportsCancellation = false)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public bool IsAsync { get; } = isAsync;
    public string? ParameterProperty { get; } = parameterProperty;
    public bool AcceptsParameter { get; } = acceptsParameter;

    /// <summary>Typed structured-result model, or null when the command has no result.</summary>
    public RustViewModelDescriptor? ResultDescriptor { get; } = resultDescriptor;

    public bool SupportsProgress { get; } = supportsProgress;
    public bool SupportsCancellation { get; } = supportsCancellation;

    public RustViewModelCommandDescriptor(int id, string name, bool isAsync, string? parameterProperty)
        : this(id, name, isAsync, parameterProperty, false)
    {
    }
}

public sealed class RustViewModelDescriptor(
    int id,
    string name,
    IReadOnlyList<RustViewModelPropertyDescriptor> properties,
    IReadOnlyList<RustViewModelCollectionDescriptor> collections,
    IReadOnlyList<RustViewModelCommandDescriptor> commands,
    IReadOnlyList<RustViewModelMapDescriptor>? maps = null)
{
    public int Id { get; } = id;
    public string Name { get; } = name;
    public IReadOnlyList<RustViewModelPropertyDescriptor> Properties { get; } = properties;
    public IReadOnlyList<RustViewModelCollectionDescriptor> Collections { get; } = collections;
    public IReadOnlyList<RustViewModelCommandDescriptor> Commands { get; } = commands;

    /// <summary>Observable keyed maps declared by this model.</summary>
    public IReadOnlyList<RustViewModelMapDescriptor> Maps { get; } = maps ?? [];
}

[GeneratedComClass]
public sealed partial class ReflectableRustViewModelAdapter :
    IAvnRustVmSink,
    IAvnRustVmSink2,
    IAvnRustVmSink3,
    IRustVmBatchTarget,
    IRustVmTableSelectionBatchTarget,
    INotifyPropertyChanged,
    INotifyDataErrorInfo,
    IReflectableType,
    IDisposable
{
    private const int InvalidArgument = unchecked((int)0x80070057);
    private readonly IAvnRustViewModel _model;
    private readonly Action<Action> _dispatch;
    private readonly Dictionary<int, RuntimeProperty> _propertiesById = [];
    private readonly Dictionary<int, RuntimeCollection> _collectionsById = [];
    private readonly Dictionary<int, DelegateCommand> _commandsById = [];
    private readonly Dictionary<string, string> _errors = new(StringComparer.Ordinal);
    private readonly RustVmInboundWriteTracker _inboundWrites = new();
    private readonly Dictionary<string, RuntimeMember> _membersByName =
        new(StringComparer.Ordinal);
    private readonly TypeInfo _typeInfo;
    private readonly RustVmBatchCoordinator _batch;
    private readonly Action<Action>? _post;

    /// <summary>
    /// Creates an adapter that dispatches and posts through
    /// <see cref="Dispatcher.UIThread"/>.
    /// </summary>
    public ReflectableRustViewModelAdapter(
        IAvnRustViewModel model,
        RustViewModelDescriptor descriptor)
        : this(model, descriptor, null, null)
    {
    }

    /// <summary>
    /// Creates an adapter with a custom synchronous dispatch for the legacy
    /// v1/v2 sink path. This overload is kept as a distinct CLR signature (not
    /// an optional parameter) so already-compiled callers keep binding to it.
    /// </summary>
    public ReflectableRustViewModelAdapter(
        IAvnRustViewModel model,
        RustViewModelDescriptor descriptor,
        Action<Action>? dispatch)
        : this(model, descriptor, dispatch, null)
    {
    }

    /// <summary>
    /// Creates an adapter with a custom synchronous <paramref name="dispatch"/>
    /// for the legacy v1/v2 sink path and a custom nonblocking
    /// <paramref name="post"/> for batch submission.
    /// </summary>
    public ReflectableRustViewModelAdapter(
        IAvnRustViewModel model,
        RustViewModelDescriptor descriptor,
        Action<Action>? dispatch,
        Action<Action>? post)
    {
        _model = model ?? throw new ArgumentNullException(nameof(model));
        ArgumentNullException.ThrowIfNull(descriptor);
        _dispatch = dispatch ?? Dispatch;
        _post = post;
        _batch = new RustVmBatchCoordinator(this, post);

        foreach (var property in descriptor.Properties)
        {
            var runtimeProperty = new RuntimeProperty(property);
            _propertiesById.Add(property.Id, runtimeProperty);
            _membersByName.Add(
                property.Name,
                new RuntimeMember(
                    property.Name,
                    ValueType(property),
                    property.Writable,
                    () => runtimeProperty.Value,
                    value => SetProperty(runtimeProperty, value)));
        }

        foreach (var collection in descriptor.Collections)
        {
            var runtimeCollection = new RuntimeCollection(collection);
            _collectionsById.Add(collection.Id, runtimeCollection);
            _membersByName.Add(
                collection.Name,
                new RuntimeMember(
                    collection.Name,
                    typeof(ObservableCollection<object?>),
                    false,
                    () => runtimeCollection.Items,
                    null));
        }

        foreach (var command in descriptor.Commands)
        {
            var value = new DelegateCommand(parameter => Execute(command, parameter));
            _commandsById.Add(command.Id, value);
            _membersByName.Add(
                command.Name,
                new RuntimeMember(
                    command.Name,
                    typeof(ICommand),
                    false,
                    () => value,
                    null));
        }

        _typeInfo = new ReflectableTypeInfo(_membersByName);
        try
        {
            Check(_model.Attach(this));
        }
        catch
        {
            try
            {
                _model.Detach();
            }
            catch
            {
            }
            DisposeNestedAdapters();
            throw;
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public event EventHandler<DataErrorsChangedEventArgs>? ErrorsChanged;

    public bool HasErrors => _errors.Count > 0;

    public TypeInfo GetTypeInfo() => _typeInfo;

    public IEnumerable GetErrors(string? propertyName) =>
        propertyName is not null && _errors.TryGetValue(propertyName, out var message)
            ? new[] { message }
            : Array.Empty<string>();

    public object? GetMemberValue(string memberName)
    {
        ArgumentException.ThrowIfNullOrEmpty(memberName);
        return _membersByName.TryGetValue(memberName, out var member)
            ? member.GetValue()
            : throw new MissingMemberException(
                $"Rust view model does not define member '{memberName}'.");
    }

    public void SetMemberValue(string memberName, object? value)
    {
        ArgumentException.ThrowIfNullOrEmpty(memberName);
        if (!_membersByName.TryGetValue(memberName, out var member))
        {
            throw new MissingMemberException(
                $"Rust view model does not define member '{memberName}'.");
        }
        if (!member.CanWrite)
            throw new InvalidOperationException($"Rust view-model member '{memberName}' is read-only.");
        member.SetValue(value);
    }

    public int SetString(int propertyId, string? value) =>
        ApplyProperty(propertyId, RustViewModelValueKind.String, value);

    public int SetInteger(int propertyId, long value) =>
        ApplyProperty(propertyId, RustViewModelValueKind.Integer, value);

    public int SetBoolean(int propertyId, int value) =>
        ApplyProperty(propertyId, RustViewModelValueKind.Boolean, value != 0);

    public int SetDouble(int propertyId, double value) =>
        ApplyProperty(propertyId, RustViewModelValueKind.Double, value);

    public int AddString(int collectionId, string? value) => InsertStringCore(collectionId, null, value);

    public int InsertString(int collectionId, int index, string? value) => InsertStringCore(collectionId, index, value);

    public int ReplaceString(int collectionId, int index, string? value)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var collection) ||
            collection.Descriptor.ElementKind != RustViewModelValueKind.String)
        {
            return InvalidArgument;
        }
        return Apply(() =>
        {
            if ((uint)index >= (uint)collection.Items.Count)
                return InvalidArgument;
            collection.Items[index] = value ?? "";
            return 0;
        });
    }

    public int AddModel(int collectionId, IAvnRustViewModel? model) => InsertModelCore(collectionId, null, model);

    public int InsertModel(int collectionId, int index, IAvnRustViewModel? model) => InsertModelCore(collectionId, index, model);

    public int ReplaceModel(int collectionId, int index, IAvnRustViewModel? model)
    {
        if (model is null ||
            !_collectionsById.TryGetValue(collectionId, out var collection) ||
            collection.Descriptor.ElementKind != RustViewModelValueKind.Model)
        {
            return InvalidArgument;
        }
        return Apply(() =>
        {
            if ((uint)index >= (uint)collection.Items.Count)
                return InvalidArgument;
            var previous = collection.Items[index] as ReflectableRustViewModelAdapter;
            collection.Items[index] =
                new ReflectableRustViewModelAdapter(model, collection.Descriptor.ElementDescriptor!, _dispatch, _post);
            previous?.Dispose();
            return 0;
        });
    }

    public int RemoveAt(int collectionId, int index)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var collection))
            return InvalidArgument;
        return Apply(() =>
        {
            if ((uint)index >= (uint)collection.Items.Count)
                return InvalidArgument;
            var removed = collection.Items[index];
            collection.Items.RemoveAt(index);
            (removed as ReflectableRustViewModelAdapter)?.Dispose();
            return 0;
        });
    }

    public int MoveItem(int collectionId, int fromIndex, int toIndex)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var collection))
            return InvalidArgument;
        return Apply(() =>
        {
            if ((uint)fromIndex >= (uint)collection.Items.Count || (uint)toIndex >= (uint)collection.Items.Count)
                return InvalidArgument;
            collection.Items.Move(fromIndex, toIndex);
            return 0;
        });
    }

    public int ClearCollection(int collectionId)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var collection))
            return InvalidArgument;
        return Apply(() =>
        {
            foreach (var item in collection.Items)
                (item as ReflectableRustViewModelAdapter)?.Dispose();
            collection.Items.Clear();
        });
    }

    public int SetNull(int propertyId)
    {
        var inbound = _inboundWrites.MarkPublication(propertyId);
        if (!_propertiesById.TryGetValue(propertyId, out var property) || !property.Descriptor.Nullable)
            return InvalidArgument;
        return Apply(() =>
        {
            if (property.Value is null)
                return;
            _inboundWrites.CommitPublication(propertyId, inbound);
            property.Value = null;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(property.Descriptor.Name));
        });
    }

    public int SetModel(int propertyId, IAvnRustViewModel? model)
    {
        if (!_propertiesById.TryGetValue(propertyId, out var property) ||
            property.Descriptor.Kind != RustViewModelValueKind.Model)
        {
            return InvalidArgument;
        }
        return Apply(() =>
        {
            var previous = property.Value as ReflectableRustViewModelAdapter;
            property.Value = model is null
                ? null
                : new ReflectableRustViewModelAdapter(model, property.Descriptor.NestedDescriptor!, _dispatch, _post);
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(property.Descriptor.Name));
            previous?.Dispose();
        });
    }

    public int SetCommandEnabled(int commandId, int enabled)
    {
        if (!_commandsById.TryGetValue(commandId, out var command))
            return InvalidArgument;
        return Apply(() => command.SetEnabled(enabled != 0));
    }

    public int SetPropertyError(int propertyId, string? message)
    {
        if (!_propertiesById.TryGetValue(propertyId, out var property))
            return InvalidArgument;
        return Apply(() => SetError(property.Descriptor.Name, message));
    }

    /// <summary>
    /// Enqueues exactly one UI work item and deliberately does not inspect
    /// <paramref name="batch"/> or complete it on this COM call stack. This is the
    /// worker-safe publication path; the legacy sink members above remain
    /// synchronous only for ABI compatibility.
    /// </summary>
    public int SubmitBatch(IAvnRustVmUpdateBatch? batch) => _batch.Submit(batch);

    /// <summary>
    /// Detaches the model and disposes every nested adapter exactly once. When a
    /// batch notification triggers this re-entrantly, the gate defers the
    /// cleanup until the batch's commit and notifications have finished, so the
    /// batch never publishes into a half-detached adapter.
    /// </summary>
    public void Dispose() => _batch.Dispose(DisposeCore);

    private void DisposeCore()
    {
        try
        {
            Check(_model.Detach());
        }
        finally
        {
            DisposeNestedAdapters();
        }
    }

    private void DisposeNestedAdapters()
    {
        foreach (var property in _propertiesById.Values)
        {
            if (property.Descriptor.Kind == RustViewModelValueKind.Model)
                TryDispose(property.Value as ReflectableRustViewModelAdapter);
        }
        foreach (var collection in _collectionsById.Values)
        {
            if (collection.Descriptor.ElementKind != RustViewModelValueKind.Model)
                continue;
            foreach (var item in collection.Items)
                TryDispose(item as ReflectableRustViewModelAdapter);
        }
    }

    private static void TryDispose(IDisposable? value)
    {
        try
        {
            value?.Dispose();
        }
        catch
        {
        }
    }

    private int InsertStringCore(int collectionId, int? index, string? value)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var collection) ||
            collection.Descriptor.ElementKind != RustViewModelValueKind.String)
        {
            return InvalidArgument;
        }
        return Apply(() =>
        {
            var target = index ?? collection.Items.Count;
            if ((uint)target > (uint)collection.Items.Count)
                return InvalidArgument;
            collection.Items.Insert(target, value ?? "");
            return 0;
        });
    }

    private int InsertModelCore(int collectionId, int? index, IAvnRustViewModel? model)
    {
        if (model is null ||
            !_collectionsById.TryGetValue(collectionId, out var collection) ||
            collection.Descriptor.ElementKind != RustViewModelValueKind.Model)
        {
            return InvalidArgument;
        }
        return Apply(() =>
        {
            var target = index ?? collection.Items.Count;
            if ((uint)target > (uint)collection.Items.Count)
                return InvalidArgument;
            collection.Items.Insert(
                target,
                new ReflectableRustViewModelAdapter(model, collection.Descriptor.ElementDescriptor!, _dispatch, _post));
            return 0;
        });
    }

    private int ApplyProperty(int propertyId, RustViewModelValueKind wireKind, object? rawValue)
    {
        var inbound = _inboundWrites.MarkPublication(propertyId);
        if (!_propertiesById.TryGetValue(propertyId, out var property) ||
            WireKind(property.Descriptor.Kind) != wireKind)
        {
            return InvalidArgument;
        }

        if (property.Descriptor.Kind == RustViewModelValueKind.Enum &&
            (property.EnumType is not { } enumType || !Enum.IsDefined(enumType, rawValue!)))
        {
            return InvalidArgument;
        }

        object? value = property.Descriptor.Kind switch
        {
            RustViewModelValueKind.Enum => Enum.ToObject(property.EnumType!, rawValue!),
            RustViewModelValueKind.String when !property.Descriptor.Nullable => rawValue ?? "",
            _ => rawValue,
        };

        return Apply(() =>
        {
            _inboundWrites.CommitPublication(propertyId, inbound);
            if (Equals(property.Value, value))
                return;

            property.Value = value;
            PropertyChanged?.Invoke(
                this,
                new PropertyChangedEventArgs(property.Descriptor.Name));
        });
    }

    private int Apply(Action action) => Apply(() =>
    {
        action();
        return 0;
    });

    private int Apply(Func<int> action)
    {
        if (_batch.IsClosed)
            return 0;

        var hresult = 0;
        void ApplyIfAlive()
        {
            if (!_batch.IsClosed)
            {
                try
                {
                    hresult = action();
                }
                catch
                {
                    hresult = unchecked((int)0x80004005);
                }
            }
        }

        try
        {
            _dispatch(ApplyIfAlive);
        }
        catch
        {
            return unchecked((int)0x80004005);
        }

        return hresult;
    }

    private void SetError(string propertyName, string? message)
    {
        if (RustVmBatchErrors.Set(_errors, propertyName, message))
            ErrorsChanged?.Invoke(this, new DataErrorsChangedEventArgs(propertyName));
    }

    bool IRustVmBatchTarget.TryGetProperty(int propertyId, out RustVmBatchProperty property)
    {
        if (!_propertiesById.TryGetValue(propertyId, out var runtime))
        {
            property = default;
            return false;
        }

        var kind = runtime.Descriptor.Kind;
        property = new RustVmBatchProperty(
            runtime.Descriptor.Name,
            BatchWireKind(kind),
            // Nested models are always clearable; the descriptor only tracks
            // nullability for scalars.
            runtime.Descriptor.Nullable || kind == RustViewModelValueKind.Model,
            kind == RustViewModelValueKind.Enum);
        return true;
    }

    bool IRustVmBatchTarget.TryGetCollection(int collectionId, out RustVmBatchCollectionInfo collection)
    {
        if (!_collectionsById.TryGetValue(collectionId, out var runtime))
        {
            collection = default;
            return false;
        }

        collection = new RustVmBatchCollectionInfo(
            runtime.Descriptor.Name,
            BatchWireKind(runtime.Descriptor.ElementKind),
            runtime.Items);
        return true;
    }

    bool IRustVmBatchTarget.TryGetCommand(int commandId, out IRustVmBatchCommand command)
    {
        if (_commandsById.TryGetValue(commandId, out var runtime))
        {
            command = runtime;
            return true;
        }

        command = null!;
        return false;
    }

    bool IRustVmBatchTarget.IsEnumValueDefined(int propertyId, long value) =>
        _propertiesById.TryGetValue(propertyId, out var property) &&
        property.EnumType is { } enumType &&
        Enum.IsDefined(enumType, value);

    IDisposable IRustVmBatchTarget.CreateNestedProperty(int propertyId, IAvnRustViewModel model) =>
        new ReflectableRustViewModelAdapter(
            model, _propertiesById[propertyId].Descriptor.NestedDescriptor!, _dispatch, _post);

    IDisposable IRustVmBatchTarget.CreateNestedElement(int collectionId, IAvnRustViewModel model) =>
        new ReflectableRustViewModelAdapter(
            model, _collectionsById[collectionId].Descriptor.ElementDescriptor!, _dispatch, _post);

    bool IRustVmBatchTarget.CommitProperty(int propertyId, in RustVmBatchValue value, out IDisposable? replaced)
    {
        replaced = null;
        var property = _propertiesById[propertyId];
        var next = value.Kind switch
        {
            RustVmUpdateKind.SetString => property.Descriptor.Nullable
                ? value.Text
                : value.Text ?? "",
            RustVmUpdateKind.SetInteger => property.EnumType is { } enumType
                ? Enum.ToObject(enumType, value.Integer)
                : value.Integer,
            RustVmUpdateKind.SetBoolean => value.Boolean,
            RustVmUpdateKind.SetDouble => value.Double,
            RustVmUpdateKind.SetNull => null,
            RustVmUpdateKind.SetModel => value.Model,
            _ => property.Value,
        };

        if (Equals(property.Value, next))
            return false;
        replaced = property.Value as ReflectableRustViewModelAdapter;
        property.Value = next;
        return true;
    }

    bool IRustVmBatchTarget.CommitError(string propertyName, string? message) =>
        RustVmBatchErrors.Set(_errors, propertyName, message);

    bool IRustVmTableSelectionBatchTarget.IsPostCollectionPropertyNotification(
        string propertyName,
        IReadOnlySet<string> changedCollections) =>
        _collectionsById.Values.Any(collection =>
            changedCollections.Contains(collection.Descriptor.Name) &&
            collection.Descriptor.Table?.Selection is { } selection &&
            (string.Equals(selection.SelectedIndexProperty, propertyName, StringComparison.Ordinal) ||
             string.Equals(selection.SelectedKeyProperty, propertyName, StringComparison.Ordinal)));

    void IRustVmBatchTarget.RaisePropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    void IRustVmBatchTarget.RaiseErrorsChanged(string propertyName) =>
        ErrorsChanged?.Invoke(this, new DataErrorsChangedEventArgs(propertyName));

    private static RustVmValueWireKind BatchWireKind(RustViewModelValueKind kind) => kind switch
    {
        RustViewModelValueKind.String => RustVmValueWireKind.String,
        RustViewModelValueKind.Integer or RustViewModelValueKind.Enum => RustVmValueWireKind.Integer,
        RustViewModelValueKind.Boolean => RustVmValueWireKind.Boolean,
        RustViewModelValueKind.Double => RustVmValueWireKind.Double,
        RustViewModelValueKind.Model => RustVmValueWireKind.Model,
        _ => RustVmValueWireKind.None,
    };

    private void SetProperty(RuntimeProperty property, object? value)
    {
        if (!property.Descriptor.Writable)
            throw new InvalidOperationException(
                $"Rust view-model property '{property.Descriptor.Name}' is read-only.");

        var acceptedValue = property.Descriptor.Kind switch
        {
            RustViewModelValueKind.String when property.Descriptor.Nullable => value as string,
            RustViewModelValueKind.String => Convert.ToString(value, CultureInfo.InvariantCulture) ?? "",
            RustViewModelValueKind.Integer => Convert.ToInt64(value, CultureInfo.InvariantCulture),
            RustViewModelValueKind.Boolean => Convert.ToBoolean(value, CultureInfo.InvariantCulture),
            RustViewModelValueKind.Double => Convert.ToDouble(value, CultureInfo.InvariantCulture),
            RustViewModelValueKind.Enum => ConvertEnumValue(property, value),
            _ => throw new ArgumentOutOfRangeException(),
        };
        if (Equals(property.Value, acceptedValue))
            return;

        var previous = property.Value;
        var inbound = _inboundWrites.Begin(property.Descriptor.Id);
        try
        {
            var result = property.Descriptor.Kind switch
            {
                RustViewModelValueKind.String =>
                    _model.SetString(property.Descriptor.Id, (string?)acceptedValue),
                RustViewModelValueKind.Integer =>
                    _model.SetInteger(property.Descriptor.Id, (long)acceptedValue!),
                RustViewModelValueKind.Boolean =>
                    _model.SetBoolean(property.Descriptor.Id, (bool)acceptedValue! ? 1 : 0),
                RustViewModelValueKind.Double =>
                    _model.SetDouble(property.Descriptor.Id, (double)acceptedValue!),
                RustViewModelValueKind.Enum =>
                    _model.SetInteger(
                        property.Descriptor.Id,
                        Convert.ToInt64(acceptedValue, CultureInfo.InvariantCulture)),
                _ => throw new ArgumentOutOfRangeException(),
            };
            Check(result);
            if (!_inboundWrites.WasPublished(inbound))
            {
                _inboundWrites.CommitLocal(property.Descriptor.Id);
                property.Value = acceptedValue;
                PropertyChanged?.Invoke(
                    this,
                    new PropertyChangedEventArgs(property.Descriptor.Name));
            }
        }
        catch
        {
            if (_inboundWrites.ShouldRollback(inbound))
            {
                _inboundWrites.CommitLocal(property.Descriptor.Id);
                property.Value = previous;
                PropertyChanged?.Invoke(
                    this,
                    new PropertyChangedEventArgs(property.Descriptor.Name));
            }
            throw;
        }
        finally { _inboundWrites.End(inbound); }
    }

    private static object ConvertEnumValue(RuntimeProperty property, object? value)
    {
        var enumType = property.EnumType
            ?? throw new InvalidOperationException(
                $"Enum property '{property.Descriptor.Name}' has no concrete enum type.");
        var converted = value is not null && value.GetType() == enumType
            ? value
            : Enum.ToObject(enumType, Convert.ToInt64(value, CultureInfo.InvariantCulture));
        if (!Enum.IsDefined(enumType, converted))
            throw new ArgumentOutOfRangeException(nameof(value), value, $"Invalid value for enum '{enumType.Name}'.");
        return converted;
    }


    private void Execute(RustViewModelCommandDescriptor command, object? commandParameter)
    {
        var parameter = command.ParameterProperty is null
            ? command.AcceptsParameter ? commandParameter as string : null
            : (string?)_membersByName[command.ParameterProperty].GetValue();
        Check(command.IsAsync
            ? _model.BeginAsync(command.Id, parameter)
            : _model.Execute(command.Id, parameter));
    }

    private static RustViewModelValueKind WireKind(RustViewModelValueKind kind) =>
        kind == RustViewModelValueKind.Enum ? RustViewModelValueKind.Integer : kind;

    private static Type ValueType(RustViewModelPropertyDescriptor descriptor) => descriptor.Kind switch
    {
        RustViewModelValueKind.String => typeof(string),
        RustViewModelValueKind.Integer => typeof(long),
        RustViewModelValueKind.Boolean => typeof(bool),
        RustViewModelValueKind.Double => typeof(double),
        RustViewModelValueKind.Enum => descriptor.InitialValue?.GetType() ?? typeof(long),
        RustViewModelValueKind.Model => typeof(ReflectableRustViewModelAdapter),
        _ => throw new ArgumentOutOfRangeException(nameof(descriptor)),
    };

    private static void Dispatch(Action action)
    {
        if (Dispatcher.UIThread.CheckAccess())
            action();
        else
            Dispatcher.UIThread.Invoke(action);
    }

    private static void Check(int hresult)
    {
        if (hresult < 0)
            Marshal.ThrowExceptionForHR(hresult);
    }

    private sealed class RuntimeProperty(RustViewModelPropertyDescriptor descriptor)
    {
        public RustViewModelPropertyDescriptor Descriptor { get; } = descriptor;

        /// <summary>
        /// The concrete generated enum type for an enum-backed property, captured
        /// from the descriptor's initial value. It is resolved once here so enum
        /// domain checks never depend on the property's current (possibly null)
        /// value.
        /// </summary>
        public Type? EnumType { get; } = descriptor.Kind == RustViewModelValueKind.Enum
            ? descriptor.InitialValue?.GetType()
            : null;

        public object? Value { get; set; } = descriptor.InitialValue;
    }

    private sealed class RuntimeCollection(RustViewModelCollectionDescriptor descriptor)
    {
        public RustViewModelCollectionDescriptor Descriptor { get; } = descriptor;
        public BatchObservableCollection<object?> Items { get; } = [];
    }

    private sealed class RuntimeMember(
        string name,
        Type propertyType,
        bool canWrite,
        Func<object?> getValue,
        Action<object?>? setValue)
    {
        public string Name { get; } = name;
        public Type PropertyType { get; } = propertyType;
        public bool CanWrite { get; } = canWrite;
        public object? GetValue() => getValue();
        public void SetValue(object? value) => setValue!(value);
    }

    private sealed class ReflectableTypeInfo(
        IReadOnlyDictionary<string, RuntimeMember> members)
        : TypeDelegator(typeof(ReflectableRustViewModelAdapter))
    {
        [UnconditionalSuppressMessage(
            "Trimming",
            "IL2094",
            Justification = "Dynamic Rust members are returned from explicit metadata, not the delegated CLR type.")]
        protected override PropertyInfo? GetPropertyImpl(
            string name,
            BindingFlags bindingAttr,
            Binder? binder,
            Type? returnType,
            Type[]? types,
            ParameterModifier[]? modifiers)
        {
            if (!members.TryGetValue(name, out var member) ||
                returnType is not null && returnType != member.PropertyType ||
                types is { Length: > 0 })
            {
                return null;
            }

            return new ReflectablePropertyInfo(member);
        }

        [UnconditionalSuppressMessage(
            "Trimming",
            "IL2094",
            Justification = "Dynamic Rust members are returned from explicit metadata, not the delegated CLR type.")]
        public override PropertyInfo[] GetProperties(BindingFlags bindingAttr)
        {
            var properties = new PropertyInfo[members.Count];
            var index = 0;
            foreach (var member in members.Values)
                properties[index++] = new ReflectablePropertyInfo(member);
            return properties;
        }
    }

    private sealed class ReflectablePropertyInfo(RuntimeMember member) : PropertyInfo
    {
        public override PropertyAttributes Attributes => PropertyAttributes.None;
        public override bool CanRead => true;
        public override bool CanWrite => member.CanWrite;
        public override Type PropertyType => member.PropertyType;
        public override Type DeclaringType => typeof(ReflectableRustViewModelAdapter);
        public override string Name => member.Name;
        public override Type ReflectedType => typeof(ReflectableRustViewModelAdapter);

        public override MethodInfo[] GetAccessors(bool nonPublic) => [];
        public override MethodInfo? GetGetMethod(bool nonPublic) => null;
        public override ParameterInfo[] GetIndexParameters() => [];
        public override MethodInfo? GetSetMethod(bool nonPublic) => null;
        public override object[] GetCustomAttributes(bool inherit) => [];
        public override object[] GetCustomAttributes(Type attributeType, bool inherit) => [];
        public override bool IsDefined(Type attributeType, bool inherit) => false;

        public override object? GetValue(
            object? obj,
            BindingFlags invokeAttr,
            Binder? binder,
            object?[]? index,
            CultureInfo? culture)
        {
            ValidateTarget(obj);
            return member.GetValue();
        }

        public override void SetValue(
            object? obj,
            object? value,
            BindingFlags invokeAttr,
            Binder? binder,
            object?[]? index,
            CultureInfo? culture)
        {
            ValidateTarget(obj);
            if (!member.CanWrite)
                throw new ArgumentException($"Property '{member.Name}' is read-only.");
            member.SetValue(value);
        }

        private static void ValidateTarget(object? target)
        {
            if (target is not ReflectableRustViewModelAdapter)
                throw new TargetException("The target is not a Rust view-model adapter.");
        }
    }

    public sealed class DelegateCommand(Action<object?> execute) : ICommand, IRustVmBatchCommand
    {
        private bool _canExecute = true;

        public DelegateCommand(Action execute) : this(_ => execute())
        {
        }

        public event EventHandler? CanExecuteChanged;
        public bool CanExecute(object? parameter) => _canExecute;
        public void Execute(object? parameter) => execute(parameter);

        public void SetEnabled(bool value)
        {
            if (SetEnabledCore(value))
                RaiseCanExecuteChanged();
        }

        public bool SetEnabledCore(bool enabled)
        {
            if (_canExecute == enabled)
                return false;
            _canExecute = enabled;
            return true;
        }

        public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);
    }
}
