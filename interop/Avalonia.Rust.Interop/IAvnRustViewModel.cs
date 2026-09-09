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
}

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
}

/// <summary>
/// A second, independently versioned sink interface. Carries the transport
/// added for nested view models, nullable values, collection insert/remove/
/// replace/move/clear, command <c>CanExecute</c> state, and validation-error
/// projection, without widening the <see cref="IAvnRustVmSink"/> vtable. A
/// generated adapter always implements both; Rust queries for this interface
/// once at attach time and surfaces an explicit ABI error if it is missing
/// rather than silently degrading.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D26")]
public partial interface IAvnRustVmSink2
{
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
}

/// <summary>
/// Immutable, versioned update-batch capability.  This interface is deliberately
/// separate from the v1 and v2 sink vtables: applications that publish from a
/// worker must use this capability rather than the synchronous per-member calls.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D27")]
public partial interface IAvnRustVmSink3
{
    /// <summary>
    /// Enqueues an immutable batch. This call never applies the batch or invokes
    /// the batch object; the adapter reads it later from one UI dispatcher item.
    /// </summary>
    [PreserveSig]
    int SubmitBatch(IAvnRustVmUpdateBatch? batch);
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
}

/// <summary>
/// The batch's separately versioned ownership-commit capability. It is a
/// distinct IID and vtable rather than an extension of
/// <see cref="IAvnRustVmUpdateBatch"/>, so the immutable batch contract that
/// already shipped is untouched.
///
/// The adapter calls <see cref="CommitOwnership"/> exactly once, immediately
/// after every notification-free managed state store and strictly before any
/// <c>PropertyChanged</c>/<c>ErrorsChanged</c>/<c>CanExecuteChanged</c>/collection
/// Reset is published. That ordering is what keeps nested ownership consistent
/// when a notification observer synchronously publishes further nested updates:
/// the batch's own nested handles are already reconciled on the producer side,
/// so the observer's updates apply on top of them instead of racing them.
///
/// A batch that is rejected during validation or staging, or that completes as
/// stale, cancelled or in error before the managed commit, never receives this
/// call and simply drops its candidate handles.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2A")]
public partial interface IAvnRustVmUpdateBatch2
{
    /// <summary>
    /// Transfers the batch's nested ownership to the producer. Called at most
    /// once per batch; the implementation must be idempotent-safe by consuming
    /// its callback.
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
/// Stage 30 sink capability: observable keyed maps, structured command
/// results, async command progress, and windowed range publication. It is a
/// new IID and vtable rather than an extension of
/// <see cref="IAvnRustVmSink2"/>/<see cref="IAvnRustVmSink3"/>, so every
/// already-published sink contract is untouched and a host that has not been
/// regenerated simply fails <c>QueryInterface</c> for this IID.
///
/// A map key is transported in both representations because the *schema*, not
/// the call site, decides which one is meaningful: a map declared with a
/// string key reads <c>stringKey</c> and ignores <c>integerKey</c>, and vice
/// versa. Generated named APIs never expose that encoding to application
/// authors.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2B")]
public partial interface IAvnRustVmSink4
{
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
}

/// <summary>
/// Scalar-number collection capability: appending, inserting and replacing
/// <c>long</c>/<c>double</c> elements of an <c>ObservableCollection&lt;long&gt;</c>
/// or <c>ObservableCollection&lt;double&gt;</c> projection.
///
/// It is a new IID and vtable rather than an extension of
/// <see cref="IAvnRustVmSink"/>/<see cref="IAvnRustVmSink2"/>, so every
/// already-published sink contract is untouched and a host that has not been
/// regenerated simply fails <c>QueryInterface</c> for this IID instead of
/// silently dropping updates.
///
/// Removal, movement and clearing are deliberately absent: they carry no
/// element value, so the already-published
/// <see cref="IAvnRustVmSink2.RemoveAt"/>,
/// <see cref="IAvnRustVmSink2.MoveItem"/> and
/// <see cref="IAvnRustVmSink2.ClearCollection"/> apply to a scalar-number
/// collection exactly as they do to a string or model one.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2F")]
public partial interface IAvnRustVmSink5
{
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
/// Stage 30 range-source capability implemented by Rust and queried from
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

/// <summary>
/// Stage 30 view-model capability implemented by Rust and queried from
/// <see cref="IAvnRustViewModel"/>: tracked async command invocation and
/// cancellation. Kept separate from <see cref="IAvnRustViewModel"/> so the
/// published vtable is untouched.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D2E")]
public partial interface IAvnRustViewModel2
{
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
