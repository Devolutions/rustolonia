using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Logging;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Rust;

/// <summary>
/// The shared staged, transactional batch engine used by both the IR-generated
/// adapters and <see cref="ReflectableRustViewModelAdapter"/>.
///
/// One submission runs in four strictly ordered phases:
/// <list type="number">
/// <item>decode - every operation and every getter HRESULT is checked;</item>
/// <item>plan - every id, exact wire kind, nullable eligibility, enum domain,
/// command id, collection element kind and simulated ordered index is validated
/// against a copy of the current collection contents, so nothing is mutated
/// until the whole batch is known to be applicable;</item>
/// <item>stage - every nested adapter the batch installs is created and
/// attached off to the side. A failure disposes the adapters staged so far and
/// leaves target state and notifications completely untouched;</item>
/// <item>commit then publish - fields, commands, errors and collection contents
/// are stored with no externally observable notification, the producer's nested
/// ownership is transferred through
/// <see cref="IAvnRustVmUpdateBatch2.CommitOwnership"/>, and only then are
/// coalesced <c>PropertyChanged</c>/<c>ErrorsChanged</c>/<c>CanExecuteChanged</c>
/// and at most one collection Reset per changed collection published.</item>
/// </list>
/// </summary>
public sealed class RustVmBatchCoordinator
{
    private const int InvalidArgument = unchecked((int)0x80070057);
    private const int Failure = unchecked((int)0x80004005);
    private const int NoInterface = unchecked((int)0x80004002);

    private readonly IRustVmBatchTarget _target;
    private readonly Action<Action> _post;
    private readonly RustVmBatchGate _gate = new();
    private long _lastGeneration = -1;

    public RustVmBatchCoordinator(IRustVmBatchTarget target, Action<Action>? post = null)
    {
        _target = target ?? throw new ArgumentNullException(nameof(target));
        _post = post ?? (action => Dispatcher.UIThread.Post(action));
    }

    /// <summary>True once disposal was requested, even while cleanup is deferred.</summary>
    public bool IsClosed => _gate.IsClosed;

    /// <summary>
    /// Enqueues one immutable batch. Deliberately does not read
    /// <paramref name="batch"/> or complete it on the submitting stack: a Rust
    /// worker never re-enters managed presentation state synchronously.
    /// </summary>
    public int Submit(IAvnRustVmUpdateBatch? batch)
    {
        if (batch is null)
            return InvalidArgument;
        try
        {
            _post(() => Apply(batch));
            return 0;
        }
        catch
        {
            // Completion is posted too, so the submitter's stack never calls back into Rust.
            try
            {
                _post(() => RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, Failure));
            }
            catch
            {
                // The dispatcher is gone; Rust's completion channel resolves when the batch drops.
            }
            return Failure;
        }
    }

    /// <summary>Requests disposal, deferring <paramref name="cleanup"/> past an active batch.</summary>
    public void Dispose(Action cleanup) => _gate.Dispose(cleanup);

    private void Apply(IAvnRustVmUpdateBatch batch)
    {
        switch (_gate.TryEnterBatch())
        {
            case RustVmBatchEntry.Disposed:
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Cancelled);
                return;
            case RustVmBatchEntry.Reentrant:
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, Failure);
                return;
        }

        try
        {
            ApplyCore(batch);
        }
        finally
        {
            _gate.ExitBatch();
        }
    }

    private void ApplyCore(IAvnRustVmUpdateBatch batch)
    {
        var plan = new BatchPlan();
        IAvnRustVmUpdateBatch2 ownership;
        long generation;
        try
        {
            var hr = batch.GetGeneration(out generation);
            if (hr < 0)
            {
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }

            // Equal generations are stale too: duplicates get a deterministic
            // outcome and the highest generation wins regardless of arrival order.
            if (generation <= _lastGeneration)
            {
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Stale);
                return;
            }

            // The ownership-commit capability is required, and it is resolved
            // before anything is decoded or staged: a producer that cannot hand
            // over nested ownership at the right point must fail with zero
            // mutation rather than leave the two sides' nested state divergent.
            if (batch is not IAvnRustVmUpdateBatch2 batch2)
            {
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, NoInterface);
                return;
            }
            ownership = batch2;

            hr = Decode(batch, out var entries);
            if (hr < 0)
            {
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }

            hr = BuildPlan(entries, plan);
            if (hr < 0)
            {
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }

            hr = Stage(plan);
            if (hr < 0)
            {
                plan.DisposeStaged();
                RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, hr);
                return;
            }
        }
        catch (Exception error)
        {
            // Nothing has been committed on any path that can reach here, so the
            // batch is rejected whole and anything already staged is released.
            plan.DisposeStaged();
            RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, HResultOf(error));
            return;
        }

        // From here the batch is applied. Observers may throw, and disposal may
        // be requested re-entrantly, but neither un-commits anything.
        try
        {
            Commit(plan);
        }
        catch (Exception error)
        {
            // A target's notification-free store threw, which its contract
            // forbids. State may already be partially stored, so this is
            // reported as an error without claiming a rollback that did not
            // happen - and the generation still advances, because the target is
            // no longer in its pre-batch state. Adapters this batch displaced or
            // staged but never installed are still released here so a target bug
            // cannot strand attached nested models.
            _lastGeneration = generation;
            plan.DisposeUninstalled();
            plan.DisposeOrphans();
            RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Error, HResultOf(error));
            return;
        }

        _lastGeneration = generation;
        // Ownership transfers between the state commit and the notifications, so
        // an observer that synchronously publishes further nested updates sees a
        // producer whose nested handles already match the committed state.
        CommitOwnership(ownership);
        Publish(plan);
        plan.DisposeOrphans();
        RustVmBatchReader.Complete(batch, RustVmBatchOutcome.Applied);
    }

    private void CommitOwnership(IAvnRustVmUpdateBatch2 ownership)
    {
        int hr;
        try
        {
            hr = ownership.CommitOwnership();
        }
        catch (Exception error)
        {
            hr = HResultOf(error);
        }

        if (hr >= 0)
            return;

        // The state is committed, so this is not reported as a failed batch. It
        // is logged because the producer's nested bookkeeping may now lag the
        // published state.
        Logger.TryGet(LogEventLevel.Error, LogArea.Binding)?.Log(
            _target,
            "A Rust view-model batch committed but its producer refused ownership transfer: 0x{Result:X8}",
            hr);
    }

    private static int HResultOf(Exception error) =>
        error.HResult < 0 ? error.HResult : Failure;

    private static int Decode(IAvnRustVmUpdateBatch batch, out BatchEntry[] entries)
    {
        entries = [];
        var hr = batch.GetOperationCount(out var count);
        if (hr < 0)
            return hr;
        if (count < 0)
            return InvalidArgument;

        var decoded = new BatchEntry[count];
        for (var index = 0; index < count; index++)
        {
            hr = batch.GetOperation(index, out var operation);
            if (hr < 0)
                return hr;
            if (operation is null)
                return InvalidArgument;
            hr = DecodeOperation(batch, index, operation, out decoded[index]);
            if (hr < 0)
                return hr;
        }

        entries = decoded;
        return 0;
    }

    private static int DecodeOperation(
        IAvnRustVmUpdateBatch batch,
        int index,
        IAvnRustVmUpdateOperation operation,
        out BatchEntry entry)
    {
        entry = default;
        var hr = operation.GetKind(out var kind);
        if (hr < 0)
            return hr;
        if (!Enum.IsDefined(typeof(RustVmUpdateKind), kind))
            return InvalidArgument;
        entry.Kind = (RustVmUpdateKind)kind;

        hr = operation.GetTargetId(out entry.Target);
        if (hr < 0)
            return hr;
        hr = operation.GetIndex(out entry.Index);
        if (hr < 0)
            return hr;
        hr = operation.GetIndex2(out entry.Index2);
        if (hr < 0)
            return hr;
        hr = operation.GetInteger(out entry.Integer);
        if (hr < 0)
            return hr;
        hr = operation.GetDouble(out entry.Double);
        if (hr < 0)
            return hr;
        hr = operation.GetBoolean(out entry.Boolean);
        if (hr < 0)
            return hr;
        hr = RustVmBatchReader.ReadText(operation, out entry.Text);
        if (hr < 0)
            return hr;
        hr = operation.GetModel(out entry.Model);
        if (hr < 0)
            return hr;

        if (entry.Kind is not (RustVmUpdateKind.ReplaceStringSnapshot or RustVmUpdateKind.ReplaceModelSnapshot))
            return 0;

        hr = batch.GetSnapshotItemCount(index, out var items);
        if (hr < 0)
            return hr;
        if (items < 0)
            return InvalidArgument;

        if (entry.Kind == RustVmUpdateKind.ReplaceStringSnapshot)
        {
            var strings = new string[items];
            for (var item = 0; item < items; item++)
            {
                hr = RustVmBatchReader.ReadSnapshotText(batch, index, item, out strings[item]);
                if (hr < 0)
                    return hr;
            }
            entry.Strings = strings;
            return 0;
        }

        var models = new IAvnRustViewModel[items];
        for (var item = 0; item < items; item++)
        {
            hr = batch.GetSnapshotModel(index, item, out var model);
            if (hr < 0)
                return hr;
            if (model is null)
                return InvalidArgument;
            models[item] = model;
        }
        entry.Models = models;
        return 0;
    }

    /// <summary>
    /// Validates the whole batch and folds it into a per-target plan. Nothing on
    /// the target is mutated: collection ordering is simulated on a working copy
    /// so every index is checked against the state that operation would really see.
    /// </summary>
    private int BuildPlan(BatchEntry[] entries, BatchPlan plan)
    {
        foreach (var entry in entries)
        {
            switch (entry.Kind)
            {
                case RustVmUpdateKind.SetString:
                case RustVmUpdateKind.SetInteger:
                case RustVmUpdateKind.SetBoolean:
                case RustVmUpdateKind.SetDouble:
                case RustVmUpdateKind.SetNull:
                case RustVmUpdateKind.SetModel:
                {
                    if (!_target.TryGetProperty(entry.Target, out var property))
                        return InvalidArgument;
                    var expected = entry.Kind switch
                    {
                        RustVmUpdateKind.SetString => RustVmValueWireKind.String,
                        RustVmUpdateKind.SetInteger => RustVmValueWireKind.Integer,
                        RustVmUpdateKind.SetBoolean => RustVmValueWireKind.Boolean,
                        RustVmUpdateKind.SetDouble => RustVmValueWireKind.Double,
                        RustVmUpdateKind.SetModel => RustVmValueWireKind.Model,
                        _ => property.WireKind,
                    };
                    if (property.WireKind != expected)
                        return InvalidArgument;
                    if (entry.Kind == RustVmUpdateKind.SetNull && !property.Nullable)
                        return InvalidArgument;
                    if (entry.Kind == RustVmUpdateKind.SetInteger && property.IsEnum &&
                        !_target.IsEnumValueDefined(entry.Target, entry.Integer))
                    {
                        return InvalidArgument;
                    }
                    if (entry.Kind == RustVmUpdateKind.SetModel && entry.Model is null)
                        return InvalidArgument;

                    var value = plan.Property(entry.Target, property);
                    value.Kind = entry.Kind;
                    value.Text = entry.Kind == RustVmUpdateKind.SetString
                        ? entry.Text
                        : null;
                    value.Integer = entry.Integer;
                    value.Double = entry.Double;
                    value.Boolean = entry.Boolean != 0;
                    // Later writes win, so an earlier nested model in the same
                    // batch is never attached at all.
                    value.Pending = entry.Kind == RustVmUpdateKind.SetModel
                        ? new PendingModel(entry.Model!)
                        : null;
                    break;
                }

                case RustVmUpdateKind.SetPropertyError:
                {
                    if (!_target.TryGetProperty(entry.Target, out var property))
                        return InvalidArgument;
                    // A nonzero boolean is the explicit "clear this error" flag;
                    // it is how an empty message stays distinguishable from none.
                    plan.Error(entry.Target, property.Name).Message =
                        entry.Boolean != 0 ? null : entry.Text;
                    break;
                }

                case RustVmUpdateKind.SetCommandEnabled:
                {
                    if (!_target.TryGetCommand(entry.Target, out var command))
                        return InvalidArgument;
                    plan.Command(entry.Target, command).Enabled = entry.Boolean != 0;
                    break;
                }

                default:
                {
                    if (!_target.TryGetCollection(entry.Target, out var collection))
                        return InvalidArgument;
                    var hr = PlanCollection(entry, plan.Collection(entry.Target, collection));
                    if (hr < 0)
                        return hr;
                    break;
                }
            }
        }

        return 0;
    }

    private static int PlanCollection(in BatchEntry entry, CollectionPlan plan)
    {
        var items = plan.Contents;
        var strings = plan.Info.ElementKind == RustVmValueWireKind.String;
        var models = plan.Info.ElementKind == RustVmValueWireKind.Model;
        switch (entry.Kind)
        {
            case RustVmUpdateKind.AddString:
                if (!strings)
                    return InvalidArgument;
                items.Add(entry.Text ?? "");
                break;
            case RustVmUpdateKind.InsertString:
                if (!strings || (uint)entry.Index > (uint)items.Count)
                    return InvalidArgument;
                items.Insert(entry.Index, entry.Text ?? "");
                break;
            case RustVmUpdateKind.ReplaceString:
                if (!strings || (uint)entry.Index >= (uint)items.Count)
                    return InvalidArgument;
                items[entry.Index] = entry.Text ?? "";
                break;
            case RustVmUpdateKind.AddModel:
                if (!models || entry.Model is null)
                    return InvalidArgument;
                items.Add(new PendingModel(entry.Model));
                break;
            case RustVmUpdateKind.InsertModel:
                if (!models || entry.Model is null || (uint)entry.Index > (uint)items.Count)
                    return InvalidArgument;
                items.Insert(entry.Index, new PendingModel(entry.Model));
                break;
            case RustVmUpdateKind.ReplaceModel:
                if (!models || entry.Model is null || (uint)entry.Index >= (uint)items.Count)
                    return InvalidArgument;
                items[entry.Index] = new PendingModel(entry.Model);
                break;
            case RustVmUpdateKind.RemoveAt:
                if ((uint)entry.Index >= (uint)items.Count)
                    return InvalidArgument;
                items.RemoveAt(entry.Index);
                break;
            case RustVmUpdateKind.MoveItem:
                if ((uint)entry.Index >= (uint)items.Count || (uint)entry.Index2 >= (uint)items.Count)
                    return InvalidArgument;
                var moved = items[entry.Index];
                items.RemoveAt(entry.Index);
                items.Insert(entry.Index2, moved);
                break;
            case RustVmUpdateKind.ClearCollection:
                items.Clear();
                break;
            case RustVmUpdateKind.ReplaceStringSnapshot:
                if (!strings || entry.Strings is null)
                    return InvalidArgument;
                items.Clear();
                foreach (var value in entry.Strings)
                    items.Add(value);
                break;
            case RustVmUpdateKind.ReplaceModelSnapshot:
                if (!models || entry.Models is null)
                    return InvalidArgument;
                items.Clear();
                foreach (var model in entry.Models)
                    items.Add(new PendingModel(model));
                break;
            default:
                return InvalidArgument;
        }

        plan.Touched = true;
        return 0;
    }

    /// <summary>
    /// Attaches every nested adapter the plan installs, before anything is
    /// committed. Only models that survive the folded plan are attached.
    /// </summary>
    private int Stage(BatchPlan plan)
    {
        foreach (var property in plan.Properties)
        {
            if (property.Pending is null)
                continue;
            try
            {
                var staged = _target.CreateNestedProperty(property.Id, property.Pending.Model);
                property.Staged = staged;
                plan.AddStaged(staged);
            }
            catch (Exception error)
            {
                return HResultOf(error);
            }
        }

        foreach (var collection in plan.Collections)
        {
            var contents = collection.Contents;
            for (var index = 0; index < contents.Count; index++)
            {
                if (contents[index] is not PendingModel pending)
                    continue;
                try
                {
                    var staged = _target.CreateNestedElement(collection.Id, pending.Model);
                    contents[index] = staged;
                    collection.HasStaged = true;
                    plan.AddStaged(staged);
                }
                catch (Exception error)
                {
                    return HResultOf(error);
                }
            }
        }

        return 0;
    }

    /// <summary>Stores all validated state without any externally observable notification.</summary>
    private void Commit(BatchPlan plan)
    {
        foreach (var property in plan.Properties)
        {
            var value = new RustVmBatchValue(
                property.Kind,
                property.Text,
                property.Integer,
                property.Double,
                property.Boolean,
                property.Staged);
            if (_target.CommitProperty(property.Id, in value, out var replaced))
            {
                property.Changed = true;
                plan.MarkInstalled(property.Staged);
            }
            else if (property.Staged is not null)
            {
                plan.Orphans.Add(property.Staged);
                plan.MarkInstalled(property.Staged);
            }
            if (replaced is not null)
                plan.Orphans.Add(replaced);
        }

        foreach (var command in plan.Commands)
            command.Changed = command.Command.SetEnabledCore(command.Enabled);

        foreach (var error in plan.Errors)
            error.Changed = _target.CommitError(error.Name, error.Message);

        foreach (var collection in plan.Collections)
        {
            // A collection holding freshly staged adapters is always considered
            // changed: skipping it would strand adapters this batch attached.
            if (!collection.Touched || (!collection.HasStaged && Unchanged(collection)))
                continue;
            if (collection.Info.ElementKind == RustVmValueWireKind.Model)
                CollectOrphans(collection, plan.Orphans);
            collection.Info.Items.SetContents(collection.Contents);
            foreach (var value in collection.Contents)
                plan.MarkInstalled(value);
            collection.Changed = true;
        }

        // A Reset clears a selecting control even when Rust retained the same
        // key/index. Force the accompanying authoritative selection
        // notification so it can restore that unchanged value after Reset.
        var changedCollections = plan.Collections
            .Where(collection => collection.Changed)
            .Select(collection => collection.Info.Name)
            .ToHashSet(StringComparer.Ordinal);
        foreach (var property in plan.Properties)
        {
            if (IsPostCollectionPropertyNotification(property.Schema.Name, changedCollections))
                property.Changed = true;
        }
    }

    private static bool Unchanged(CollectionPlan collection)
    {
        var items = collection.Info.Items;
        if (items.Count != collection.Contents.Count)
            return false;
        for (var index = 0; index < collection.Contents.Count; index++)
        {
            if (!Equals(items.GetAt(index), collection.Contents[index]))
                return false;
        }
        return true;
    }

    private static void CollectOrphans(CollectionPlan collection, List<IDisposable> orphans)
    {
        var kept = new HashSet<object>(ReferenceEqualityComparer.Instance);
        foreach (var value in collection.Contents)
        {
            if (value is not null)
                kept.Add(value);
        }

        var items = collection.Info.Items;
        for (var index = 0; index < items.Count; index++)
        {
            if (items.GetAt(index) is IDisposable existing && !kept.Contains(existing))
                orphans.Add(existing);
        }
    }

    /// <summary>
    /// Publishes the coalesced notifications for an already committed batch. An
    /// observer that throws cannot roll anything back, so each notification is
    /// isolated: the remaining ones are still published and the failure is
    /// logged rather than reported as a batch error.
    /// </summary>
    private void Publish(BatchPlan plan)
    {
        var changedCollections = plan.Collections
            .Where(collection => collection.Changed)
            .Select(collection => collection.Info.Name)
            .ToHashSet(StringComparer.Ordinal);

        foreach (var property in plan.Properties)
        {
            if (property.Changed &&
                !IsPostCollectionPropertyNotification(property.Schema.Name, changedCollections))
                Notify(property.Schema.Name, () => _target.RaisePropertyChanged(property.Schema.Name));
        }

        foreach (var collection in plan.Collections)
        {
            if (collection.Changed)
                Notify(collection.Info.Name, collection.Info.Items.RaiseReset);
        }

        foreach (var property in plan.Properties)
        {
            if (property.Changed &&
                IsPostCollectionPropertyNotification(property.Schema.Name, changedCollections))
            {
                Notify(property.Schema.Name, () => _target.RaisePropertyChanged(property.Schema.Name));
            }
        }

        foreach (var command in plan.Commands)
        {
            if (command.Changed)
                Notify(null, command.Command.RaiseCanExecuteChanged);
        }

        foreach (var error in plan.Errors)
        {
            if (error.Changed)
                Notify(error.Name, () => _target.RaiseErrorsChanged(error.Name));
        }
    }

    private void Notify(string? member, Action publish)
    {
        try
        {
            publish();
        }

        catch (Exception error)
        {
            Logger.TryGet(LogEventLevel.Error, LogArea.Binding)?.Log(
                _target,
                "Rust view-model batch notification for '{Member}' threw after the batch was committed: {Error}",
                member ?? "<command>",
                error);
        }
    }

    private bool IsPostCollectionPropertyNotification(
        string propertyName,
        IReadOnlySet<string> changedCollections) =>
        _target is IRustVmTableSelectionBatchTarget tableTarget &&
        tableTarget.IsPostCollectionPropertyNotification(propertyName, changedCollections);

    private sealed class PendingModel(IAvnRustViewModel model)
    {
        public IAvnRustViewModel Model { get; } = model;
    }

    private struct BatchEntry
    {
        public RustVmUpdateKind Kind;
        public int Target;
        public int Index;
        public int Index2;
        public long Integer;
        public double Double;
        public int Boolean;
        public string Text;
        public IAvnRustViewModel? Model;
        public string[]? Strings;
        public IAvnRustViewModel[]? Models;
    }

    private sealed class PropertyPlan(int id, RustVmBatchProperty schema)
    {
        public int Id { get; } = id;
        public RustVmBatchProperty Schema { get; } = schema;
        public RustVmUpdateKind Kind;
        public string? Text;
        public long Integer;
        public double Double;
        public bool Boolean;
        public PendingModel? Pending;
        public IDisposable? Staged;
        public bool Changed;
    }

    private sealed class CollectionPlan(int id, RustVmBatchCollectionInfo info)
    {
        public int Id { get; } = id;
        public RustVmBatchCollectionInfo Info { get; } = info;
        public List<object?> Contents { get; } = Snapshot(info.Items);
        public bool Touched;
        public bool HasStaged;
        public bool Changed;

        private static List<object?> Snapshot(IRustVmBatchCollection items)
        {
            var values = new List<object?>(items.Count);
            for (var index = 0; index < items.Count; index++)
                values.Add(items.GetAt(index));
            return values;
        }
    }

    private sealed class CommandPlan(IRustVmBatchCommand command)
    {
        public IRustVmBatchCommand Command { get; } = command;
        public bool Enabled;
        public bool Changed;
    }

    private sealed class ErrorPlan(string name)
    {
        public string Name { get; } = name;
        public string? Message;
        public bool Changed;
    }

    /// <summary>
    /// The folded batch. Every collection preserves first-touch order so the
    /// published notification order is deterministic.
    /// </summary>
    private sealed class BatchPlan
    {
        private readonly Dictionary<int, PropertyPlan> _properties = [];
        private readonly Dictionary<int, CollectionPlan> _collections = [];
        private readonly Dictionary<int, CommandPlan> _commands = [];
        private readonly Dictionary<int, ErrorPlan> _errors = [];
        private readonly List<IDisposable> _staged = [];

        /// <summary>Staged adapters the commit has not yet handed to the target.</summary>
        private readonly HashSet<object> _uninstalled = new(ReferenceEqualityComparer.Instance);

        public List<PropertyPlan> Properties { get; } = [];
        public List<CollectionPlan> Collections { get; } = [];
        public List<CommandPlan> Commands { get; } = [];
        public List<ErrorPlan> Errors { get; } = [];
        public List<IDisposable> Orphans { get; } = [];

        public void AddStaged(IDisposable adapter)
        {
            _staged.Add(adapter);
            _uninstalled.Add(adapter);
        }

        public void MarkInstalled(object? value)
        {
            if (value is not null)
                _uninstalled.Remove(value);
        }

        public PropertyPlan Property(int id, RustVmBatchProperty schema)
        {
            if (_properties.TryGetValue(id, out var plan))
                return plan;
            plan = new PropertyPlan(id, schema);
            _properties.Add(id, plan);
            Properties.Add(plan);
            return plan;
        }

        public CollectionPlan Collection(int id, RustVmBatchCollectionInfo info)
        {
            if (_collections.TryGetValue(id, out var plan))
                return plan;
            plan = new CollectionPlan(id, info);
            _collections.Add(id, plan);
            Collections.Add(plan);
            return plan;
        }

        public CommandPlan Command(int id, IRustVmBatchCommand command)
        {
            if (_commands.TryGetValue(id, out var plan))
                return plan;
            plan = new CommandPlan(command);
            _commands.Add(id, plan);
            Commands.Add(plan);
            return plan;
        }

        public ErrorPlan Error(int id, string name)
        {
            if (_errors.TryGetValue(id, out var plan))
                return plan;
            plan = new ErrorPlan(name);
            _errors.Add(id, plan);
            Errors.Add(plan);
            return plan;
        }

        /// <summary>Releases every staged adapter, used when the batch never commits.</summary>
        public void DisposeStaged()
        {
            foreach (var staged in _staged)
                TryDispose(staged);
            _staged.Clear();
            _uninstalled.Clear();
        }

        /// <summary>Releases staged adapters an interrupted commit never installed.</summary>
        public void DisposeUninstalled()
        {
            foreach (var staged in _uninstalled)
                TryDispose((IDisposable)staged);
            _uninstalled.Clear();
        }

        public void DisposeOrphans()
        {
            foreach (var orphan in Orphans)
                TryDispose(orphan);
            Orphans.Clear();
        }

        private static void TryDispose(IDisposable value)
        {
            try
            {
                value.Dispose();
            }
            catch (Exception error)
            {
                Logger.TryGet(LogEventLevel.Error, LogArea.Binding)?.Log(
                    value,
                    "Disposing a Rust nested view-model adapter failed: {Error}",
                    error);
            }
        }
    }
}
