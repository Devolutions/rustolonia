using System;
using System.Collections.Generic;
using Avalonia.Input;
using Avalonia.Platform.Storage;

namespace Avalonia.Host.Desktop;

/// <summary>Which incoming drag notification a delivery describes.</summary>
public enum DesktopDropEventKind
{
    Enter = 0,
    Over = 1,
    Leave = 2,
    Drop = 3,
}

/// <summary>
/// One drag notification, already detached from the managed event that produced
/// it, ready to be delivered asynchronously.
/// </summary>
public sealed record DesktopDropEvent(
    DesktopDropEventKind Kind,
    DragDropEffects AllowedEffects,
    DragDropEffects EffectiveEffects,
    IReadOnlyList<StorageItemSnapshot> Items);

/// <summary>
/// The synchronous half of incoming drag-and-drop: everything that has to be
/// decided while the platform drag loop is still on the stack.
/// </summary>
/// <remarks>
/// Rust is never asked to negotiate an effect synchronously. Calling into an
/// external consumer from inside a platform drag loop would let arbitrary Rust
/// code stall the compositor (and, on Windows, run inside an OLE modal loop), so
/// the subscriber declares a conservative accepted-effect mask once, at
/// subscription time, and the host answers the platform immediately with the
/// intersection of that mask and the platform's allowed effects. The
/// notification itself, including the file list, is delivered asynchronously.
/// </remarks>
public static class DesktopFileDrop
{
    /// <summary>
    /// Computes the effect reported back to the platform. A notification that
    /// carries no file/folder items is always refused, so a drag of unrelated
    /// content does not appear droppable.
    /// </summary>
    public static DragDropEffects NegotiateEffect(
        DragDropEffects allowedEffects,
        DragDropEffects acceptedEffects,
        bool hasItems)
    {
        if (!hasItems)
            return DragDropEffects.None;
        return allowedEffects & acceptedEffects;
    }

    /// <summary>
    /// Captures the storage items of a drag payload. Returns an empty list when
    /// the payload carries no file format at all.
    /// </summary>
    public static IReadOnlyList<StorageItemSnapshot> CaptureItems(IDataTransfer? dataTransfer)
    {
        if (dataTransfer is null)
            return Array.Empty<StorageItemSnapshot>();

        IStorageItem[]? files;
        try
        {
            files = dataTransfer.TryGetFiles();
        }
        catch (Exception)
        {
            // A platform payload can fail to materialize (revoked handle, remote
            // item); an undroppable drag is the correct outcome, not a crash in
            // the drag loop.
            return Array.Empty<StorageItemSnapshot>();
        }

        if (files is null || files.Length == 0)
            return Array.Empty<StorageItemSnapshot>();

        var snapshots = new List<StorageItemSnapshot>(files.Length);
        foreach (var file in files)
        {
            try
            {
                snapshots.Add(StorageItemSnapshot.FromStorageItem(file));
            }
            catch (Exception)
            {
                // Skip individual items the platform cannot describe.
            }
        }

        return snapshots;
    }

    /// <summary>
    /// Builds the notification for one managed drag event and, as a side effect,
    /// the effect that must be written back to the event arguments.
    /// </summary>
    public static DesktopDropEvent Prepare(
        DesktopDropEventKind kind,
        DragDropEffects allowedEffects,
        DragDropEffects acceptedEffects,
        IDataTransfer? dataTransfer) =>
        PrepareFrom(
            kind,
            allowedEffects,
            acceptedEffects,
            kind == DesktopDropEventKind.Leave
                ? Array.Empty<StorageItemSnapshot>()
                : CaptureItems(dataTransfer));

    /// <summary>
    /// Builds the notification from an already-captured payload.
    /// </summary>
    /// <remarks>
    /// <c>DragOver</c> fires continuously while the pointer moves, so the
    /// payload is captured once per drag and reused. Re-materializing it on
    /// every notification would allocate a fresh storage item (and re-probe its
    /// local path) for every file, hundreds of times a second, on the UI thread
    /// with the platform drag loop still on the stack — exactly what this design
    /// avoids doing to the consumer.
    /// </remarks>
    public static DesktopDropEvent PrepareFrom(
        DesktopDropEventKind kind,
        DragDropEffects allowedEffects,
        DragDropEffects acceptedEffects,
        IReadOnlyList<StorageItemSnapshot> items)
    {
        if (kind == DesktopDropEventKind.Leave)
        {
            return new DesktopDropEvent(
                kind,
                allowedEffects,
                DragDropEffects.None,
                Array.Empty<StorageItemSnapshot>());
        }

        return new DesktopDropEvent(
            kind,
            allowedEffects,
            NegotiateEffect(allowedEffects, acceptedEffects, items.Count > 0),
            items);
    }
}
