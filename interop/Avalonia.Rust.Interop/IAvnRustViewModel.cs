using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Rust.Interop;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D24")]
public partial interface IAvnRustViewModel
{
    [PreserveSig]
    int Attach(IAvnRustVmSink? sink);

    [PreserveSig]
    int Detach();

    [PreserveSig]
    int SetString(int propertyId, string? value);

    [PreserveSig]
    int SetInteger(int propertyId, long value);

    [PreserveSig]
    int SetBoolean(int propertyId, int value);

    [PreserveSig]
    int SetDouble(int propertyId, double value);

    [PreserveSig]
    int Execute(int commandId, string? parameter);

    [PreserveSig]
    int BeginAsync(int commandId, string? parameter);

    /// <summary>
    /// Starts an async command and returns a handle identifying that specific
    /// invocation. The handle is never reused, so a cancellation for an
    /// already-finished invocation is ignored instead of aborting its successor.
    /// </summary>
    [PreserveSig]
    int BeginAsyncTracked(int commandId, string? parameter, out long operationId);

    /// <summary>Requests cancellation of one in-flight invocation. Never blocks.</summary>
    [PreserveSig]
    int CancelAsync(int commandId, long operationId);
}

/// <summary>
/// The managed adapter's sink a Rust view model publishes into. Generated
/// adapters implement every member; the reflectable adapter reports the
/// optional transports it lacks through <see cref="GetCapabilities"/>.
///
/// A map key is transported in both representations because the schema, not
/// the call site, decides which one is meaningful: a map declared with a string
/// key reads <c>stringKey</c> and ignores <c>integerKey</c>, and vice versa.
/// Removal, movement and clearing of a scalar-number collection use the same
/// <see cref="RemoveAt"/>, <see cref="MoveItem"/> and
/// <see cref="ClearCollection"/> as string and model collections.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D25")]
public partial interface IAvnRustVmSink
{
    [PreserveSig]
    int SetString(int propertyId, string? value);

    [PreserveSig]
    int SetInteger(int propertyId, long value);

    [PreserveSig]
    int SetBoolean(int propertyId, int value);

    [PreserveSig]
    int SetDouble(int propertyId, double value);

    [PreserveSig]
    int AddString(int collectionId, string? value);

    /// <summary>Publishes that a nullable scalar property now has no value.</summary>
    [PreserveSig]
    int SetNull(int propertyId);

    /// <summary>Publishes a nested view-model property (null clears it).</summary>
    [PreserveSig]
    int SetModel(int propertyId, IAvnRustViewModel? model);

    /// <summary>Appends a nested view-model item to a model-kind collection.</summary>
    [PreserveSig]
    int AddModel(int collectionId, IAvnRustViewModel? model);

    [PreserveSig]
    int InsertString(int collectionId, int index, string? value);

    [PreserveSig]
    int InsertModel(int collectionId, int index, IAvnRustViewModel? model);

    [PreserveSig]
    int ReplaceString(int collectionId, int index, string? value);

    [PreserveSig]
    int ReplaceModel(int collectionId, int index, IAvnRustViewModel? model);

    [PreserveSig]
    int RemoveAt(int collectionId, int index);

    [PreserveSig]
    int MoveItem(int collectionId, int fromIndex, int toIndex);

    /// <summary>Clears a collection. Managed raises a single Reset notification.</summary>
    [PreserveSig]
    int ClearCollection(int collectionId);

    /// <summary>Publishes a command's current <c>ICommand.CanExecute</c> state.</summary>
    [PreserveSig]
    int SetCommandEnabled(int commandId, int enabled);

    /// <summary>Publishes (or clears, when <paramref name="message"/> is null) a validation error for a property.</summary>
    [PreserveSig]
    int SetPropertyError(int propertyId, string? message);

    /// <summary>
    /// Enqueues an immutable batch. This call never applies the batch or invokes
    /// the batch object; the adapter reads it later from one UI dispatcher item.
    /// </summary>
    [PreserveSig]
    int SubmitBatch(IAvnRustVmUpdateBatch? batch);

    /// <summary>Inserts or replaces a string value for one map key.</summary>
    [PreserveSig]
    int MapSetString(int mapId, string? stringKey, long integerKey, string? value);

    [PreserveSig]
    int MapSetInteger(int mapId, string? stringKey, long integerKey, long value);

    [PreserveSig]
    int MapSetBoolean(int mapId, string? stringKey, long integerKey, int value);

    [PreserveSig]
    int MapSetDouble(int mapId, string? stringKey, long integerKey, double value);

    /// <summary>Inserts or replaces a nested view-model value for one map key.</summary>
    [PreserveSig]
    int MapSetModel(int mapId, string? stringKey, long integerKey, IAvnRustViewModel? value);

    /// <summary>Removes one key. Missing keys are a no-op, not an error.</summary>
    [PreserveSig]
    int MapRemove(int mapId, string? stringKey, long integerKey);

    /// <summary>Clears a map. Managed raises a single Reset notification.</summary>
    [PreserveSig]
    int MapClear(int mapId);

    /// <summary>
    /// Publishes an async command's progress. <paramref name="hasValue"/> is 0
    /// for indeterminate progress; otherwise <paramref name="value"/> is a
    /// fraction clamped to 0..1.
    /// </summary>
    [PreserveSig]
    int SetCommandProgress(int commandId, int hasValue, double value, string? message);

    /// <summary>Publishes (or clears) a command's typed structured result.</summary>
    [PreserveSig]
    int SetCommandResult(int commandId, IAvnRustViewModel? result);

    /// <summary>Publishes a command's running state, used to gate cancellation.</summary>
    [PreserveSig]
    int SetCommandRunning(int commandId, int running);

    /// <summary>
    /// Delivers one generation-stamped range batch for a windowed collection.
    /// The call queues the batch; it is decoded on the UI thread and never on
    /// the submitting (Rust worker) stack.
    /// </summary>
    [PreserveSig]
    int PublishRange(IAvnRustVmRangeBatch? batch);

    /// <summary>Appends one element to an integer-kind collection.</summary>
    [PreserveSig]
    int AddInteger(int collectionId, long value);

    [PreserveSig]
    int InsertInteger(int collectionId, int index, long value);

    [PreserveSig]
    int ReplaceInteger(int collectionId, int index, long value);

    /// <summary>Appends one element to a double-kind collection.</summary>
    [PreserveSig]
    int AddDouble(int collectionId, double value);

    [PreserveSig]
    int InsertDouble(int collectionId, int index, double value);

    [PreserveSig]
    int ReplaceDouble(int collectionId, int index, double value);

    /// <summary>
    /// Reports which optional transports this sink applies, as
    /// <see cref="RustVmSinkCapabilities"/> flags. Every member exists on every
    /// sink; one whose capability is absent returns <c>E_NOTIMPL</c>.
    /// </summary>
    [PreserveSig]
    int GetCapabilities(out int capabilities);
}

/// <summary>Optional transports reported by <see cref="IAvnRustVmSink.GetCapabilities"/>.</summary>
public static class RustVmSinkCapabilities
{
    /// <summary>Keyed maps, command results/progress/running state and windowed ranges.</summary>
    public const int RicherShapes = 1;

    /// <summary>Integer and double collection elements.</summary>
    public const int NumberCollections = 2;

    public const int All = RicherShapes | NumberCollections;
}

/// <summary>
/// Read-only nano-COM representation of a Rust-authored update batch.
/// Implementations must be immutable for their complete COM lifetime.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D28")]
public partial interface IAvnRustVmUpdateBatch
{
    [PreserveSig]
    int GetGeneration(out long generation);

    [PreserveSig]
    int GetOperationCount(out int count);

    [PreserveSig]
    int GetOperation(int index, out IAvnRustVmUpdateOperation? operation);

    [PreserveSig]
    int GetSnapshotItemCount(int operationIndex, out int count);

    [PreserveSig]
    int GetSnapshotStringLength(int operationIndex, int itemIndex, out int length);

    [PreserveSig]
    unsafe int CopySnapshotString(int operationIndex, int itemIndex, char* destination, int capacity);

    [PreserveSig]
    int GetSnapshotModel(int operationIndex, int itemIndex, out IAvnRustViewModel? model);

    /// <summary>Completes a queued batch after its UI-thread outcome is known.</summary>
    [PreserveSig]
    int Complete(int outcome, int error);

    /// <summary>
    /// Transfers the batch's nested ownership to the producer. Called at most
    /// once per batch; the implementation must be idempotent-safe by consuming
    /// its callback.
    ///
    /// The adapter calls this immediately after every notification-free managed
    /// state store and strictly before any <c>PropertyChanged</c>/
    /// <c>ErrorsChanged</c>/<c>CanExecuteChanged</c>/collection Reset is
    /// published, so an observer that synchronously publishes further nested
    /// updates applies them on top of already reconciled producer handles. A
    /// batch rejected during validation or staging, or completed as stale,
    /// cancelled or in error before the managed commit, never receives this
    /// call and simply drops its candidate handles.
    /// </summary>
    [PreserveSig]
    int CommitOwnership();
}

/// <summary>One immutable operation in an <see cref="IAvnRustVmUpdateBatch"/>.</summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D29")]
public partial interface IAvnRustVmUpdateOperation
{
    [PreserveSig]
    int GetKind(out int kind);

    [PreserveSig]
    int GetTargetId(out int targetId);

    [PreserveSig]
    int GetIndex(out int index);

    [PreserveSig]
    int GetIndex2(out int index);

    [PreserveSig]
    int GetInteger(out long value);

    [PreserveSig]
    int GetDouble(out double value);

    [PreserveSig]
    int GetBoolean(out int value);

    [PreserveSig]
    int GetTextLength(out int length);

    [PreserveSig]
    unsafe int CopyText(char* destination, int capacity);

    [PreserveSig]
    int GetModel(out IAvnRustViewModel? model);
}

/// <summary>
/// Read-only nano-COM representation of one realized range of a windowed
/// collection. Implementations must be immutable for their complete COM
/// lifetime, exactly like <see cref="IAvnRustVmUpdateBatch"/>.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2C")]
public partial interface IAvnRustVmRangeBatch
{
    /// <summary>
    /// The batch's wire kind: 0 republishes the dataset identity (generation
    /// and total count) and invalidates every realized page, 1 realizes one
    /// page. They are distinct kinds rather than an implicit "empty range
    /// means reset" so a legitimately empty page can never be mistaken for a
    /// reset.
    /// </summary>
    [PreserveSig]
    int GetKind(out int kind);

    [PreserveSig]
    int GetCollectionId(out int collectionId);

    /// <summary>The dataset generation this range was produced from.</summary>
    [PreserveSig]
    int GetGeneration(out long generation);

    /// <summary>The dataset's total element count at <see cref="GetGeneration"/>.</summary>
    [PreserveSig]
    int GetTotalCount(out long total);

    /// <summary>Index of the first element carried by this batch.</summary>
    [PreserveSig]
    int GetOffset(out long offset);

    [PreserveSig]
    int GetItemCount(out int count);

    [PreserveSig]
    int GetItemModel(int index, out IAvnRustViewModel? model);

    [PreserveSig]
    int GetItemStringLength(int index, out int length);

    [PreserveSig]
    unsafe int CopyItemString(int index, char* destination, int capacity);

    /// <summary>Completes a queued range batch after its UI-thread outcome is known.</summary>
    [PreserveSig]
    int Complete(int outcome, int error);
}

/// <summary>
/// Range-source interface implemented by Rust and queried from
/// <see cref="IAvnRustViewModel"/>. Rust owns the full dataset; managed code
/// asks for the window it can actually see. <see cref="RequestRange"/> must
/// return immediately: it posts work, it never produces a range inline and
/// never blocks the UI thread.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2D")]
public partial interface IAvnRustRangeSource
{
    /// <summary>Reads the current dataset generation and total count for a windowed collection.</summary>
    [PreserveSig]
    int GetRangeState(int collectionId, out long generation, out long totalCount);

    /// <summary>
    /// Asks Rust to realize <paramref name="length"/> elements starting at
    /// <paramref name="offset"/>. Rust rejects the request when
    /// <paramref name="generation"/> is not its current generation.
    ///
    /// The request queue is shared by every windowed collection on the model
    /// and is bounded, so accepting a new request may evict an older one.
    /// <paramref name="droppedCollectionId"/>/<paramref name="droppedOffset"/>
    /// identify the evicted request (offset -1 when nothing was evicted),
    /// because a dropped request will never produce a range batch and its
    /// owner must stop waiting for one.
    /// </summary>
    [PreserveSig]
    int RequestRange(
        int collectionId,
        long offset,
        int length,
        long generation,
        out int droppedCollectionId,
        out long droppedOffset);
}
