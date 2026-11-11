using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Rust.Interop;

namespace Avalonia.Host.Com;

/// <summary>
/// The application object Rust drives. One interface, changed in place: the
/// host and the Rust crates always ship together, so new operations are
/// appended here rather than published as a numbered successor interface.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnApplication)]
public partial interface IAvnApplication
{
    [PreserveSig]
    int Run(IAvnAppHandler? handler);

    [PreserveSig]
    int Shutdown();

    [PreserveSig]
    int GetRequestedThemeVariant(out int value);

    [PreserveSig]
    int SetRequestedThemeVariant(int value);

    [PreserveSig]
    int GetActualThemeVariant(out int value);

    [PreserveSig]
    int TryGetResource(string? key, int themeVariant, out int found, out IAvnResourceValue? value);

    [PreserveSig]
    int StartDelay(int milliseconds, IAvnAsyncCompletion? completion, out long operationId);

    [PreserveSig]
    int StartClipboardSetText(
        IAvnWindow? window,
        string? text,
        IAvnAsyncCompletion? completion,
        out long operationId);

    [PreserveSig]
    int StartClipboardGetText(
        IAvnWindow? window,
        IAvnAsyncCompletion? completion,
        out long operationId);

    [PreserveSig]
    int CancelAsyncOperation(long operationId);

    [PreserveSig]
    int CreateRustVmWindow(int viewId, IAvnRustViewModel? model, out IAvnWindow? window);

    /// <summary>
    /// Registers (or clears, when <paramref name="provider"/> is null) the
    /// single application-scoped Rust value-converter provider. A second call
    /// with a different, still-registered provider is rejected.
    /// </summary>
    [PreserveSig]
    int SetValueConverterProvider(IAvnRustValueConverterProvider? provider);

    // Desktop file integration: pickers, file drops, startup arguments and
    // file/folder activation.

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

    // Clipboard commands. Plain text write/read are StartClipboardSetText and
    // StartClipboardGetText above; file entries are read back as the same
    // immutable IAvnStorageItemList snapshots the pickers and drops produce.

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
    /// Reuses <see cref="IAvnStorageCompletion"/>, so a clipboard with no files
    /// completes successfully with an empty list rather than failing.
    /// </summary>
    [PreserveSig]
    int StartClipboardReadFiles(
        IAvnWindow? window,
        IAvnStorageCompletion? completion,
        out long operationId);

    /// <summary>
    /// Shows <paramref name="dialog"/> modally over <paramref name="owner"/> and completes
    /// through the shared async operation registry: exactly one completion, cancellable
    /// through <c>CancelAsyncOperation</c>, never blocking the UI thread. The result string
    /// is the dialog result converted by the host (null when the dialog returned no result).
    /// </summary>
    [PreserveSig]
    int StartShowDialog(
        IAvnWindow? owner,
        IAvnWindow? dialog,
        IAvnAsyncCompletion? completion,
        out long operationId);

    // Desktop shell: how the application lifetime ends, notification-area
    // (tray) icons, and the window-level settings a tray or capture tool needs.

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

    // Code-first desktop pieces. A pixel surface is a control that draws
    // Rust-owned bitmap layers and reports its raw pointer stream to a Rust
    // handler; zoom, hit testing and what to draw stay on the Rust side. Tray
    // icons declared in a view-model schema need that schema compiled into the
    // host, so CreateTrayIcon builds one, and its menu, from Rust instead.

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

    // Native-window operations.

    [PreserveSig]
    int GetNativeWindowHandle(IAvnWindow? window, out long handle, out int kind);

    [PreserveSig]
    int GetMacOSWindowNumber(IAvnWindow? window, out int number);

    [PreserveSig]
    int SetMacOSWindowIgnoresMouseEvents(IAvnWindow? window, int ignores);

    [PreserveSig]
    int ShowWindow(IAvnWindow? window, int activate);

    [PreserveSig]
    int ActivateWindow(IAvnWindow? window);

    /// <summary>
    /// Per-window popup hosting. Set before opening popups; does not migrate
    /// already-open popups.
    /// </summary>
    [PreserveSig]
    int SetWindowOverlayPopups(IAvnWindow? window, int enabled);

    /// <summary>macOS UI thread; close popups before reconfiguration. Null removes the gate.</summary>
    [PreserveSig]
    int SetNativePopupGate(IAvnWindow? window, IAvnNativePopupHandler? handler);

    /// <summary>UI thread; stale/duplicate tokens return E_INVALIDARG.</summary>
    [PreserveSig]
    int AcknowledgeNativePopup(long token);

    [PreserveSig]
    int SetTerminationHandler(IAvnTerminationHandler? handler);

    [PreserveSig]
    int CompleteTermination(long token, int accept);

    [PreserveSig]
    int SetFileDragHandler(IAvnControl? target, IAvnFileDragHandler? handler);

    /// <summary>JSON array of absolute file URLs, at least one; UI thread.</summary>
    [PreserveSig]
    int CompleteFileDrag(long token, string? fileUrlsJson);

    [PreserveSig]
    int CancelFileDrag(long token);

    /// <summary>
    /// Single-view lifetimes (browser and iOS hosts) expose one main view
    /// instead of windows and accept the root control here; desktop hosts
    /// return E_NOTIMPL.
    /// </summary>
    [PreserveSig]
    int SetMainView(IAvnControl? view);

    /// <summary>
    /// Names the application before initialization. Platform-owned lifetimes
    /// that initialize Avalonia before Rust runs return E_NOTIMPL.
    /// </summary>
    [PreserveSig]
    int SetApplicationName(string? name);

    // Code-first native menus: Avalonia's NativeMenu built from Rust. UI thread
    // only. A window menu is exported to the platform menu bar where there is
    // one (macOS) and drawn in the window by a CreateNativeMenuBar control
    // everywhere else; the control hides itself when the menu is exported, so
    // an application places one in every window unconditionally. The
    // application menu is the macOS application-named menu (About,
    // Settings…); other platforms do not show it.

    /// <summary>Creates an empty menu reporting clicks to <paramref name="handler"/>.</summary>
    [PreserveSig]
    int CreateNativeMenu(IAvnNativeMenuHandler? handler, out IAvnNativeMenu? menu);

    /// <summary>
    /// Places <paramref name="menu"/>'s items at the top of the application
    /// menu, replacing any previous one; null removes them.
    /// </summary>
    [PreserveSig]
    int SetApplicationMenu(IAvnNativeMenu? menu);

    /// <summary>
    /// Makes <paramref name="menu"/> the window's menu bar and installs its
    /// shortcuts as window key bindings; null removes both.
    /// </summary>
    [PreserveSig]
    int SetWindowMenu(IAvnWindow? window, IAvnNativeMenu? menu);

    /// <summary>Whether the platform shows the window's menu natively (outside the window).</summary>
    [PreserveSig]
    int IsNativeMenuExported(IAvnWindow? window, out int exported);

    /// <summary>
    /// A control drawing its window's menu in-window, hidden while the platform
    /// exports that menu natively.
    /// </summary>
    [PreserveSig]
    int CreateNativeMenuBar(out IAvnControl? bar);

    // Code-first declarative styles and window key bindings.

    /// <summary>
    /// Compiles and attaches a JSON-encoded style sheet to <paramref name="scopeControl"/>
    /// (or to <c>Application.Current.Styles</c> when <paramref name="scopeControl"/> is null),
    /// returning a handle removable via <see cref="RemoveStyles"/>.
    /// </summary>
    [PreserveSig]
    int ApplyStyles(IAvnControl? scopeControl, string? rulesJson, out long styleHandle);

    /// <summary>Removes a style sheet previously registered with <see cref="ApplyStyles"/>.</summary>
    [PreserveSig]
    int RemoveStyles(long styleHandle);

    /// <summary>Adds a <c>KeyBinding</c> to <paramref name="window"/>.</summary>
    [PreserveSig]
    int AddWindowKeyBinding(
        IAvnWindow? window,
        string? gesture,
        IAvnCommand? command,
        AvnVariant parameter,
        out long bindingId);

    /// <summary>Removes a <c>KeyBinding</c> previously added to <paramref name="window"/>.</summary>
    [PreserveSig]
    int RemoveWindowKeyBinding(IAvnWindow? window, long bindingId);
}
