using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 34 code-first desktop pieces. Queried as an optional interface from
/// <see cref="IAvnApplication"/>; no published vtable changes.
/// </summary>
/// <remarks>
/// <para>
/// The projection covers Avalonia's controls, but an image editor, a selection
/// overlay or a chart needs two things it cannot express: pixels that Rust
/// produced, and the raw pointer stream (press, move, release with positions).
/// A pixel surface is a control that draws Rust-owned bitmap layers and reports
/// its input to a Rust handler; everything else (zoom, hit testing, what to
/// draw) stays on the Rust side.
/// </para>
/// <para>
/// Stage 33's tray icons are declared in a view-model schema compiled into the
/// host, which a code-first application on the prebuilt host does not have.
/// <see cref="CreateTrayIcon"/> builds one, and its menu, from Rust instead.
/// </para>
/// </remarks>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication7)]
public partial interface IAvnApplication7
{
    /// <summary>
    /// Creates a pixel surface reporting its input to <paramref name="handler"/>
    /// (which may be null for a display-only surface). Place it with
    /// <see cref="IAvnPixelSurface.GetControl"/>.
    /// </summary>
    [PreserveSig]
    int CreatePixelSurface(IAvnPixelSurfaceHandler? handler, out IAvnPixelSurface? surface);

    /// <summary>
    /// Shows a notification-area icon reporting clicks to
    /// <paramref name="handler"/> (null for an icon that does nothing). The icon
    /// starts visible with no image or menu, and stays until
    /// <see cref="IAvnShellTray.Remove"/> or the application shuts down.
    /// </summary>
    [PreserveSig]
    int CreateTrayIcon(IAvnShellTrayHandler? handler, out IAvnShellTray? tray);
}
