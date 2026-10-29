using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Generated.ViewModels;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 33 desktop shell integration: the shutdown mode, tray icons and
/// window-level settings. Avalonia's own <c>ShutdownMode</c>, <c>TrayIcon</c>,
/// <c>Topmost</c> and <c>TransparencyLevelHint</c> do the work; only the
/// capture exclusion calls the platform directly, because Avalonia has no
/// equivalent.
/// </summary>
public partial class AvnApplication : IAvnApplication6
{
    private ShutdownMode _shutdownMode = ShutdownMode.OnLastWindowClose;
    private readonly List<IDisposable> _trays = new();

    public int SetShutdownMode(int mode)
    {
        if (mode is < (int)ShutdownMode.OnLastWindowClose or > (int)ShutdownMode.OnExplicitShutdown)
            return HResults.E_INVALIDARG;
        try
        {
            _shutdownMode = (ShutdownMode)mode;
            if (_lifetime is { } lifetime)
                lifetime.ShutdownMode = _shutdownMode;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int CreateRustVmTray(int trayId, IAvnRustViewModel? model)
    {
        if (model is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            _trays.Add(RustViewRegistry.CreateTray(trayId, model));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int GetPlatformHandle(IAvnWindow? window, out long handle)
    {
        long found = 0;
        var hr = WithWindow(window, managed => found = managed.TryGetPlatformHandle()?.Handle ?? 0);
        handle = found;
        return hr;
    }

    public int SetWindowTopmost(IAvnWindow? window, int topmost) =>
        WithWindow(window, managed => managed.Topmost = topmost != 0);

    public int SetWindowTransparent(IAvnWindow? window, int transparent) =>
        WithWindow(window, managed => managed.TransparencyLevelHint = transparent != 0
            ? [WindowTransparencyLevel.Transparent]
            : []);

    public int SetWindowExcludedFromCapture(IAvnWindow? window, int excluded)
    {
        if (!OperatingSystem.IsWindows())
            return HResults.E_NOTIMPL;
        var result = 0;
        var hr = WithWindow(window, managed =>
        {
            var hwnd = managed.TryGetPlatformHandle()?.Handle ?? 0;
            if (hwnd == 0)
                throw new InvalidOperationException("The window has no native handle yet; show it first.");
            if (!Win32.SetWindowDisplayAffinity(hwnd, excluded != 0 ? Win32.WDA_EXCLUDEFROMCAPTURE : Win32.WDA_NONE))
                result = Marshal.GetHRForLastWin32Error();
        });
        return hr < 0 ? hr : result;
    }

    /// <summary>Runs <paramref name="action"/> on the managed window, on the UI thread.</summary>
    private static int WithWindow(IAvnWindow? window, Action<Window> action)
    {
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var managed = (Window?)ProjectionRuntime.Unwrap(window)
                ?? throw new ObjectDisposedException(nameof(window));
            action(managed);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    /// <summary>Removes the tray icons; Avalonia also disposes them as the dispatcher shuts down.</summary>
    private void ReleaseTrays()
    {
        foreach (var tray in _trays)
        {
            try
            {
                tray.Dispose();
            }
            catch (Exception)
            {
                // Shutdown keeps going; the platform removes the icon with the process.
            }
        }
        _trays.Clear();
    }

    private static partial class Win32
    {
        public const uint WDA_NONE = 0x00;
        public const uint WDA_EXCLUDEFROMCAPTURE = 0x11;

        [LibraryImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool SetWindowDisplayAffinity(nint hwnd, uint affinity);
    }
}
