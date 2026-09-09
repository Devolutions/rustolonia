using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// What the top-level's clipboard offers, as reported by
/// <see cref="IAvnApplication4.GetClipboardCapabilities"/>.
/// </summary>
[System.Flags]
internal enum AvnClipboardCapabilityFlags
{
    None = 0,

    /// <summary>A clipboard exists on this top-level at all.</summary>
    Available = 1,

    /// <summary>
    /// The top-level also has a storage provider, so file entries written to or
    /// read from the clipboard can be resolved to storage items.
    /// </summary>
    Files = 2,
}

/// <summary>
/// Host-owned, mutable clipboard payload builder.
/// </summary>
/// <remarks>
/// Rust never implements this. It creates one through
/// <see cref="IAvnApplication4.CreateClipboardData"/> and fills it in, so this
/// direction of the ABI carries only primitives and UTF-16 strings - exactly
/// the rule stage 29 established for picker options. Building the real
/// <c>DataTransfer</c> (and resolving file URIs to storage items) happens on the
/// host side, on the UI thread, when the write actually runs.
/// </remarks>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnClipboardData)]
public partial interface IAvnClipboardData
{
    /// <summary>Sets (or clears, when null) the text payload.</summary>
    [PreserveSig]
    int SetText(string? value);

    /// <summary>
    /// Appends one file entry from an absolute URI or an absolute local path.
    /// Entries that the top-level's storage provider cannot resolve are dropped
    /// when the write runs; a write is not failed by an unresolvable entry.
    /// </summary>
    [PreserveSig]
    int AddFileUri(string? uriOrPath);
}

/// <summary>
/// Stage 31 clipboard command capability. Queried as an optional interface from
/// <see cref="IAvnApplication"/>; the published <see cref="IAvnApplication"/>,
/// <see cref="IAvnApplication2"/> and <see cref="IAvnApplication3"/> vtables are
/// untouched.
/// </summary>
/// <remarks>
/// Plain text write/read already exist on the frozen
/// <see cref="IAvnApplication"/> vtable (<c>StartClipboardSetText</c>,
/// <c>StartClipboardGetText</c>) and are not duplicated here. This capability
/// adds what stage 31 needs on top: clearing, multi-format writes and reading
/// file entries back as the same immutable
/// <see cref="IAvnStorageItemList"/> snapshots the stage 29 pickers and drops
/// already produce.
/// </remarks>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication4)]
public partial interface IAvnApplication4
{
    /// <summary>Creates an empty, host-owned clipboard payload builder.</summary>
    [PreserveSig]
    int CreateClipboardData(out IAvnClipboardData? data);

    /// <summary>
    /// Reports the top-level's clipboard capabilities as
    /// <see cref="AvnClipboardCapabilityFlags"/>.
    /// </summary>
    [PreserveSig]
    int GetClipboardCapabilities(IAvnWindow? window, out int capabilities);

    /// <summary>Writes the accumulated payload to the clipboard.</summary>
    [PreserveSig]
    int StartClipboardWrite(
        IAvnWindow? window,
        IAvnClipboardData? data,
        IAvnAsyncCompletion? completion,
        out long operationId);

    /// <summary>Clears the clipboard.</summary>
    [PreserveSig]
    int StartClipboardClear(
        IAvnWindow? window,
        IAvnAsyncCompletion? completion,
        out long operationId);

    /// <summary>
    /// Reads file entries from the clipboard as immutable storage snapshots.
    /// Reuses the published <see cref="IAvnStorageCompletion"/>, so a clipboard
    /// with no files completes successfully with an empty list rather than
    /// failing.
    /// </summary>
    [PreserveSig]
    int StartClipboardReadFiles(
        IAvnWindow? window,
        IAvnStorageCompletion? completion,
        out long operationId);
}
