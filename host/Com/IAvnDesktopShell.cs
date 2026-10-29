using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Rust.Interop;

namespace Avalonia.Host.Com;

/// <summary>
/// The separately versioned desktop shell capability, queried from
/// <c>IAvnApplication</c>: how the application lifetime ends, notification-area
/// (tray) icons, and the window-level settings a tray or capture tool needs.
/// Nothing here is ever added to an already published vtable; a consumer that
/// predates the capability reads <c>E_NOINTERFACE</c> from the query instead.
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication6)]
public partial interface IAvnApplication6
{
    /// <summary>
    /// Sets when the application exits: 0 when its last window closes (the
    /// default), 1 when its main window closes, 2 only on <c>Shutdown</c>.
    /// Takes effect at once when the application is running, otherwise when it
    /// starts.
    /// </summary>
    [PreserveSig]
    int SetShutdownMode(int mode);

    /// <summary>
    /// Shows the tray icon declared as <paramref name="trayId"/> in the view-model
    /// schema, bound to <paramref name="model"/>. The icon stays until the
    /// application shuts down.
    /// </summary>
    [PreserveSig]
    int CreateRustVmTray(int trayId, IAvnRustViewModel? model);

    /// <summary>
    /// The window's native handle (an <c>HWND</c> on Windows), or 0 when the
    /// platform window does not exist yet or has none.
    /// </summary>
    [PreserveSig]
    int GetPlatformHandle(IAvnWindow? window, out long handle);

    /// <summary>Keeps the window above every non-topmost window (non-zero) or not.</summary>
    [PreserveSig]
    int SetWindowTopmost(IAvnWindow? window, int topmost);

    /// <summary>
    /// Asks for a per-pixel transparent window (non-zero), so a transparent
    /// background shows what is behind it, or an opaque one.
    /// </summary>
    [PreserveSig]
    int SetWindowTransparent(IAvnWindow? window, int transparent);

    /// <summary>
    /// Leaves the window out of screenshots and recordings (non-zero) or not.
    /// Windows only: <c>E_NOTIMPL</c> elsewhere.
    /// </summary>
    [PreserveSig]
    int SetWindowExcludedFromCapture(IAvnWindow? window, int excluded);
}
