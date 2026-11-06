using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// What the top-level's clipboard offers, as reported by
/// <see cref="IAvnApplication.GetClipboardCapabilities"/>.
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
/// <see cref="IAvnApplication.CreateClipboardData"/> and fills it in, so this
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
