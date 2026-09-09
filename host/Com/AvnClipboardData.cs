using System.Collections.Generic;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Immutable snapshot of one clipboard payload.
/// </summary>
public sealed record ClipboardPayload(string? Text, IReadOnlyList<string> FileUris);

/// <summary>
/// Host-owned clipboard payload builder handed to Rust so that direction of the
/// ABI only carries primitives and strings.
/// </summary>
[GeneratedComClass]
public sealed partial class AvnClipboardData : IAvnClipboardData
{
    private readonly List<string> _fileUris = new();
    private string? _text;

    /// <summary>
    /// Copies the accumulated payload. A write runs against the copy, so a
    /// consumer that keeps mutating the builder after starting an operation
    /// cannot change what is being written.
    /// </summary>
    internal ClipboardPayload Snapshot() => new(_text, _fileUris.ToArray());

    public int SetText(string? value)
    {
        _text = value;
        return HResults.S_OK;
    }

    public int AddFileUri(string? uriOrPath)
    {
        if (string.IsNullOrWhiteSpace(uriOrPath))
            return HResults.E_INVALIDARG;
        _fileUris.Add(uriOrPath);
        return HResults.S_OK;
    }
}
