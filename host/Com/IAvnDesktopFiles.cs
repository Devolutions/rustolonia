using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Kind of a projected storage item. Matches
/// <c>avalonia::StorageItemKind</c> on the Rust side.
/// </summary>
internal enum AvnStorageItemKind
{
    File = 0,
    Folder = 1,
}

/// <summary>
/// Outcome tag carried by <see cref="IAvnStorageCompletion.Complete"/> when the
/// HRESULT is a success code. A user-cancelled picker is <em>not</em> an error:
/// it reports <see cref="Cancelled"/> with an empty item list, which is what
/// keeps "user dismissed the dialog" distinguishable from "the picker failed"
/// and from an aborted operation (<c>E_ABORT</c>).
/// </summary>
internal enum AvnStorageOutcome
{
    Completed = 0,
    Cancelled = 1,
}

/// <summary>
/// Drag-and-drop notification kinds delivered through
/// <see cref="IAvnFileDropHandler.OnDragEvent"/>.
/// </summary>
internal enum AvnFileDropEventKind
{
    Enter = 0,
    Over = 1,
    Leave = 2,
    Drop = 3,
}

/// <summary>
/// An immutable snapshot of one <see cref="Avalonia.Platform.Storage.IStorageItem"/>.
/// </summary>
/// <remarks>
/// Snapshots exist because a drag-and-drop <c>IDataTransfer</c> is only valid for
/// the duration of the managed event, and because a picker result must survive the
/// window that opened it. Not every storage item has a local filesystem path
/// (Android <c>content:</c> URIs, browser handles, macOS security-scoped items), so
/// <see cref="TryGetLocalPath"/> reports availability separately from
/// <see cref="GetUri"/>, which is always present.
/// </remarks>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnStorageItem)]
public partial interface IAvnStorageItem
{
    /// <summary>Gets the <see cref="AvnStorageItemKind"/> of this item.</summary>
    [PreserveSig]
    int GetKind(out int value);

    /// <summary>Gets the display name, including the extension when there is one.</summary>
    [PreserveSig]
    int GetName(out string? value);

    /// <summary>Gets the absolute URI of the item. Always available.</summary>
    [PreserveSig]
    int GetUri(out string? value);

    /// <summary>
    /// Gets the local filesystem path when the platform has one.
    /// <paramref name="found"/> is 0 for non-local items.
    /// </summary>
    [PreserveSig]
    int TryGetLocalPath(out int found, out string? value);
}

/// <summary>An ordered, immutable list of <see cref="IAvnStorageItem"/>.</summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnStorageItemList)]
public partial interface IAvnStorageItemList
{
    [PreserveSig]
    int GetCount(out int value);

    [PreserveSig]
    int GetItem(int index, out IAvnStorageItem? value);
}

/// <summary>
/// Host-owned, mutable picker options builder. Rust never implements this: it
/// creates one through <see cref="IAvnApplication3.CreatePickerOptions"/> and
/// fills it in, so the ABI carries only primitives and UTF-16 strings.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnFilePickerOptions)]
public partial interface IAvnFilePickerOptions
{
    [PreserveSig]
    int SetTitle(string? value);

    [PreserveSig]
    int SetAllowMultiple(int value);

    [PreserveSig]
    int SetSuggestedFileName(string? value);

    /// <summary>
    /// Sets the suggested start location from an absolute URI or an absolute
    /// local path. Resolution happens when the picker starts, on the UI thread.
    /// </summary>
    [PreserveSig]
    int SetSuggestedStartLocation(string? uriOrPath);

    /// <summary>
    /// Sets the suggested start location from an
    /// <see cref="Avalonia.Platform.Storage.WellKnownFolder"/> value, or -1 for none.
    /// </summary>
    [PreserveSig]
    int SetSuggestedStartWellKnownFolder(int value);

    [PreserveSig]
    int SetDefaultExtension(string? value);

    /// <summary>Sets the overwrite prompt: -1 leaves the platform default, 0/1 force it.</summary>
    [PreserveSig]
    int SetShowOverwritePrompt(int value);

    /// <summary>Appends a file type filter and returns its zero-based index.</summary>
    [PreserveSig]
    int AddFileType(string? name, out int index);

    [PreserveSig]
    int AddFileTypePattern(int index, string? pattern);

    [PreserveSig]
    int AddFileTypeMimeType(int index, string? mimeType);

    [PreserveSig]
    int AddFileTypeAppleUniformTypeIdentifier(int index, string? identifier);

    /// <summary>Preselects a previously added file type, or -1 for none.</summary>
    [PreserveSig]
    int SetSuggestedFileTypeIndex(int index);
}

/// <summary>
/// Rust-implemented completion for a storage operation. Called exactly once per
/// operation, on the UI thread, with the same operation ID that started it.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnStorageCompletion)]
public partial interface IAvnStorageCompletion
{
    [PreserveSig]
    int Complete(
        long operationId,
        int hresult,
        int outcome,
        IAvnStorageItemList? items,
        string? error);
}

/// <summary>
/// Rust-implemented incoming drag-and-drop sink.
/// </summary>
/// <remarks>
/// The host never asks Rust to negotiate a drag effect synchronously. The
/// accepted effects are configured once, at subscription time, and the host
/// intersects them with the platform's allowed effects while the managed event
/// is still on the stack. Notifications are then posted to the dispatcher, so a
/// slow or blocking Rust handler can never stall the drag loop.
/// </remarks>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnFileDropHandler)]
public partial interface IAvnFileDropHandler
{
    [PreserveSig]
    int OnDragEvent(
        long subscriptionId,
        int kind,
        int allowedEffects,
        int effectiveEffects,
        IAvnStorageItemList? items);
}

/// <summary>
/// Rust-implemented sink for lifetime activation that happens after startup
/// (macOS "open with" while already running, protocol activation, dock reopen).
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnActivationHandler)]
public partial interface IAvnActivationHandler
{
    /// <param name="kind">An <see cref="Avalonia.Controls.ApplicationLifetimes.ActivationKind"/> value.</param>
    /// <param name="items">Activated items, empty for kinds that carry none.</param>
    [PreserveSig]
    int OnActivated(int kind, IAvnStorageItemList? items);
}

/// <summary>
/// Stage 29 desktop file integration capability. Queried as an optional
/// interface from <see cref="IAvnApplication"/>; the previously published
/// <see cref="IAvnApplication"/> and <see cref="IAvnApplication2"/> vtables are
/// unchanged.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnApplication3)]
public partial interface IAvnApplication3
{
    /// <summary>Creates an empty, host-owned picker options builder.</summary>
    [PreserveSig]
    int CreatePickerOptions(out IAvnFilePickerOptions? options);

    /// <summary>
    /// Reports which pickers the window's top-level storage provider supports,
    /// as <see cref="Avalonia.Host.Desktop.AvnStorageCapabilityFlags"/> flags.
    /// </summary>
    [PreserveSig]
    int GetStorageCapabilities(IAvnWindow? window, out int capabilities);

    [PreserveSig]
    int StartOpenFilePicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId);

    [PreserveSig]
    int StartOpenFolderPicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId);

    [PreserveSig]
    int StartSaveFilePicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId);

    /// <param name="acceptedEffects">
    /// A conservative <see cref="Avalonia.Input.DragDropEffects"/> mask chosen by
    /// the subscriber; the host reports back the intersection with the platform's
    /// allowed effects and never calls Rust to negotiate.
    /// </param>
    [PreserveSig]
    int SubscribeFileDrop(
        IAvnControl? target,
        int acceptedEffects,
        IAvnFileDropHandler? handler,
        out long subscriptionId);

    [PreserveSig]
    int UnsubscribeFileDrop(long subscriptionId);

    /// <summary>Clears the startup arguments collected for the next <c>Run</c>.</summary>
    [PreserveSig]
    int ClearStartupArguments();

    /// <summary>Appends one verbatim startup argument, preserving order.</summary>
    [PreserveSig]
    int AddStartupArgument(string? value);

    [PreserveSig]
    int GetStartupArgumentCount(out int count);

    [PreserveSig]
    int GetStartupArgument(int index, out string? value);

    /// <summary>
    /// Returns the normalized, de-duplicated, order-preserving file/folder
    /// activation items derived from the startup arguments.
    /// </summary>
    [PreserveSig]
    int GetActivationItems(out IAvnStorageItemList? items);

    [PreserveSig]
    int AdviseActivation(IAvnActivationHandler? handler, out long subscriptionId);

    [PreserveSig]
    int UnadviseActivation(long subscriptionId);
}
