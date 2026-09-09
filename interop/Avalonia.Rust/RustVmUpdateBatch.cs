using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using Avalonia.Rust.Interop;

namespace Avalonia.Rust;

/// <summary>
/// Wire tags for one immutable batch operation. The numeric values are ABI and
/// match <c>avalonia::view_model::ViewModelBatch</c> on the Rust side.
/// </summary>
public enum RustVmUpdateKind
{
    SetString = 1, SetInteger, SetBoolean, SetDouble, SetNull, SetModel,
    AddString, AddModel, InsertString, InsertModel, ReplaceString, ReplaceModel,
    RemoveAt, MoveItem, ReplaceStringSnapshot, ReplaceModelSnapshot,
    SetCommandEnabled, SetPropertyError, ClearCollection,
}

/// <summary>Terminal outcome reported back to Rust for a submitted batch.</summary>
public enum RustVmBatchOutcome { Applied, Stale, Cancelled, Error }

/// <summary>
/// Wire tags for a property or collection-element value kind. A batch target
/// reports these so the shared engine can validate an operation's kind exactly
/// (not merely "some scalar") before any state is touched.
/// </summary>
public enum RustVmValueWireKind
{
    None = 0,
    String = 1,
    Integer = 2,
    Boolean = 3,
    Double = 4,
    Model = 6,
}

/// <summary>
/// The non-generic batch surface of a projected collection: the engine reads
/// the current contents to simulate ordered indices, replaces the backing
/// contents with no observable notification, and later raises exactly one
/// <see cref="NotifyCollectionChangedAction.Reset"/>.
/// </summary>
public interface IRustVmBatchCollection
{
    int Count { get; }

    object? GetAt(int index);

    /// <summary>Replaces the backing store without raising any notification.</summary>
    void SetContents(IReadOnlyList<object?> values);

    /// <summary>Raises one coalesced <c>Count</c>/<c>Item[]</c>/Reset notification.</summary>
    void RaiseReset();
}

/// <summary>
/// The batch surface of a projected command: the engine changes
/// <c>CanExecute</c> state during the notification-free commit and raises
/// <c>CanExecuteChanged</c> once afterwards.
/// </summary>
public interface IRustVmBatchCommand
{
    /// <summary>Stores the new state without notifying. Returns true when it changed.</summary>
    bool SetEnabledCore(bool enabled);

    void RaiseCanExecuteChanged();
}

public sealed class BatchObservableCollection<T> : ObservableCollection<T>, IRustVmBatchCollection
{
    /// <summary>
    /// Atomically replaces the contents and raises a single Reset. Retained for
    /// the synchronous v1/v2 sink path; batches use
    /// <see cref="IRustVmBatchCollection"/> so commit and notification stay separate.
    /// </summary>
    public void ReplaceSnapshot(IReadOnlyList<T> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        Items.Clear();
        foreach (var value in values)
            Items.Add(value);
        Reset();
    }

    int IRustVmBatchCollection.Count => Items.Count;

    object? IRustVmBatchCollection.GetAt(int index) => Items[index];

    void IRustVmBatchCollection.SetContents(IReadOnlyList<object?> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        Items.Clear();
        foreach (var value in values)
            Items.Add((T)value!);
    }

    void IRustVmBatchCollection.RaiseReset() => Reset();

    private void Reset()
    {
        OnPropertyChanged(new PropertyChangedEventArgs(nameof(Count)));
        OnPropertyChanged(new PropertyChangedEventArgs("Item[]"));
        OnCollectionChanged(new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset));
    }
}

/// <summary>IR-generated adapters implement this to atomically replace snapshots.</summary>
public interface IRustVmStringSnapshotSink
{
    int ReplaceStringSnapshot(int collectionId, IReadOnlyList<string> values);
}

/// <summary>IR-generated adapters implement this to atomically replace model snapshots.</summary>
public interface IRustVmModelSnapshotSink
{
    int ReplaceModelSnapshot(int collectionId, IReadOnlyList<IAvnRustViewModel> values);
}

/// <summary>Schema and storage description of one projected property.</summary>
/// <param name="Name">The managed member name used for notifications.</param>
/// <param name="WireKind">The exact wire kind an operation must carry to target this property.</param>
/// <param name="Nullable">True when <see cref="RustVmUpdateKind.SetNull"/> may target this property.</param>
/// <param name="IsEnum">True when an integer operation must also fall inside the enum's domain.</param>
public readonly record struct RustVmBatchProperty(
    string Name,
    RustVmValueWireKind WireKind,
    bool Nullable,
    bool IsEnum);

/// <summary>Schema and storage description of one projected collection.</summary>
/// <param name="Name">The managed member name, used for diagnostics.</param>
/// <param name="ElementKind">The exact wire kind a collection operation must carry.</param>
/// <param name="Items">The batch surface of the projected collection.</param>
public readonly record struct RustVmBatchCollectionInfo(
    string Name,
    RustVmValueWireKind ElementKind,
    IRustVmBatchCollection Items);

/// <summary>
/// One fully validated and staged property value handed to
/// <see cref="IRustVmBatchTarget.CommitProperty"/>. The target stores it in its
/// own strongly typed field and must not raise any notification.
/// </summary>
/// <param name="Kind">The scalar, null or model operation that produced this value.</param>
/// <param name="Text">The decoded text for <see cref="RustVmUpdateKind.SetString"/>.</param>
/// <param name="Integer">The decoded integer or enum value.</param>
/// <param name="Double">The decoded double value.</param>
/// <param name="Boolean">The decoded boolean value.</param>
/// <param name="Model">The staged nested adapter for <see cref="RustVmUpdateKind.SetModel"/>.</param>
public readonly record struct RustVmBatchValue(
    RustVmUpdateKind Kind,
    string? Text,
    long Integer,
    double Double,
    bool Boolean,
    object? Model);

/// <summary>
/// The transactional contract both the IR-generated adapters and the
/// reflectable adapter implement. <see cref="RustVmBatchCoordinator"/> owns
/// decoding, validation, staging, commit ordering, notification coalescing and
/// nested-adapter ownership; a target only exposes its schema and performs
/// notification-free stores.
/// </summary>
public interface IRustVmBatchTarget
{
    bool TryGetProperty(int propertyId, out RustVmBatchProperty property);

    bool TryGetCollection(int collectionId, out RustVmBatchCollectionInfo collection);

    bool TryGetCommand(int commandId, out IRustVmBatchCommand command);

    /// <summary>Checks an integer against an enum-backed property's declared domain.</summary>
    bool IsEnumValueDefined(int propertyId, long value);

    /// <summary>
    /// Creates and attaches a nested adapter for a model property without
    /// publishing it. Throws when the nested attach fails; the engine then
    /// disposes every staged adapter and leaves target state untouched.
    /// </summary>
    IDisposable CreateNestedProperty(int propertyId, IAvnRustViewModel model);

    /// <summary>Creates and attaches an unpublished nested adapter for a collection element.</summary>
    IDisposable CreateNestedElement(int collectionId, IAvnRustViewModel model);

    /// <summary>
    /// Stores a validated value with no notification. Returns true when the
    /// stored value changed; <paramref name="replaced"/> receives a nested
    /// adapter this store displaced, which the engine disposes after publishing.
    /// </summary>
    bool CommitProperty(int propertyId, in RustVmBatchValue value, out IDisposable? replaced);

    /// <summary>Stores or clears a validation error with no notification. Returns true when it changed.</summary>
    bool CommitError(string propertyName, string? message);

    void RaisePropertyChanged(string propertyName);

    void RaiseErrorsChanged(string propertyName);
}

/// <summary>
/// Optional table-selection notification capability. Kept separate from
/// <see cref="IRustVmBatchTarget"/> so existing compiled batch targets retain
/// their original contract and receive the normal notification ordering.
/// </summary>
public interface IRustVmTableSelectionBatchTarget
{
    bool IsPostCollectionPropertyNotification(string propertyName, IReadOnlySet<string> changedCollections);
}

/// <summary>Shared validation-error storage used by every batch target.</summary>
public static class RustVmBatchErrors
{
    /// <summary>Applies an error change and reports whether anything actually changed.</summary>
    public static bool Set(Dictionary<string, string> errors, string propertyName, string? message)
    {
        ArgumentNullException.ThrowIfNull(errors);
        ArgumentNullException.ThrowIfNull(propertyName);
        if (message is null)
            return errors.Remove(propertyName);
        if (errors.TryGetValue(propertyName, out var existing) &&
            string.Equals(existing, message, StringComparison.Ordinal))
        {
            return false;
        }
        errors[propertyName] = message;
        return true;
    }
}

internal static unsafe class RustVmBatchReader
{
    private const int InvalidArgument = unchecked((int)0x80070057);

    internal static int ReadText(IAvnRustVmUpdateOperation operation, out string value)
    {
        value = string.Empty;
        var hr = operation.GetTextLength(out var length);
        if (hr < 0)
            return hr;
        if (length < 0)
            return InvalidArgument;
        var buffer = new char[length + 1];
        fixed (char* pointer = buffer)
            hr = operation.CopyText(pointer, buffer.Length);
        if (hr >= 0)
            value = new string(buffer, 0, length);
        return hr;
    }

    internal static int ReadSnapshotText(IAvnRustVmUpdateBatch batch, int operation, int item, out string value)
    {
        value = string.Empty;
        var hr = batch.GetSnapshotStringLength(operation, item, out var length);
        if (hr < 0)
            return hr;
        if (length < 0)
            return InvalidArgument;
        var buffer = new char[length + 1];
        fixed (char* pointer = buffer)
            hr = batch.CopySnapshotString(operation, item, pointer, buffer.Length);
        if (hr >= 0)
            value = new string(buffer, 0, length);
        return hr;
    }

    internal static void Complete(IAvnRustVmUpdateBatch batch, RustVmBatchOutcome outcome, int error = 0)
    {
        try
        {
            _ = batch.Complete((int)outcome, error);
        }
        catch
        {
            // Rust owns the completion channel; a foreign failure here must not
            // turn an already committed batch into a managed exception.
        }
    }
}
