using System;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Rust;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>Stage 34: creates tray icons from Rust.</summary>
public partial class AvnApplication
{
    public int CreateTrayIcon(IAvnShellTrayHandler? handler, out IAvnShellTray? tray)
    {
        tray = null;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var application = Application.Current
                ?? throw new InvalidOperationException("A tray icon needs a running application.");
            var created = new AvnShellTray(application, handler);
            _trays.Add(created);
            tray = created;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}

/// <summary>
/// An Avalonia <see cref="TrayIcon"/> whose <see cref="NativeMenu"/> Rust builds
/// item by item. Avalonia only shows icons in the application's
/// <see cref="TrayIcons"/>, so the icon is added there and removed on
/// <see cref="Remove"/> (or by the application's shutdown).
/// </summary>
[GeneratedComClass]
internal sealed partial class AvnShellTray : IAvnShellTray, IDisposable
{
    // The published stage 34 contract: kinds 0-3 and flag bits 0-1 only.
    private const int TrayFlags = NativeMenuTree.FlagChecked | NativeMenuTree.FlagDisabled;

    private readonly Application _application;
    private readonly NativeMenuTree _menu;
    private IAvnShellTrayHandler? _handler;
    private TrayIcon? _icon;

    public AvnShellTray(Application application, IAvnShellTrayHandler? handler)
    {
        _application = application;
        _handler = handler;
        _menu = new NativeMenuTree(id => _handler?.OnMenuItem(id));
        // Avalonia.Native's UTF-8 tooltip marshaller does not accept null.
        _icon = new TrayIcon { Menu = _menu.Menu, ToolTipText = string.Empty, IsVisible = true };
        _icon.Clicked += (_, _) => _handler?.OnActivate();
        var icons = TrayIcon.GetIcons(application);
        if (icons is null)
        {
            icons = new TrayIcons();
            TrayIcon.SetIcons(application, icons);
        }
        icons.Add(_icon);
    }

    public int SetIcon(string? source) => Run(icon => icon.Icon = string.IsNullOrWhiteSpace(source) ? null : RustTrayAttachment.LoadIcon(source));

    public int SetToolTip(string? text) => Run(icon => icon.ToolTipText = text ?? string.Empty);

    public int SetVisible(int visible) => Run(icon => icon.IsVisible = visible != 0);

    public int ClearMenu() => Run(_ => _menu.Clear());

    public int AddMenuItem(int parent, int id, int kind, string? header, int flags)
    {
        if (id <= 0 || kind is < NativeMenuTree.KindCommand or > NativeMenuTree.KindSubmenu)
            return HResults.E_INVALIDARG;
        return Run(_ => _menu.Add(parent, id, kind, header, null, flags & TrayFlags));
    }

    public int UpdateMenuItem(int id, string? header, int flags) =>
        Run(_ => _menu.Update(id, header, flags & TrayFlags));

    public int Remove() => Run(_ => Dispose());

    public void Dispose()
    {
        _handler = null;
        if (_icon is not { } icon)
            return;
        _icon = null;
        if (TrayIcon.GetIcons(_application) is { } icons)
            icons.Remove(icon);
        icon.Dispose();
    }

    private int Run(Action<TrayIcon> action)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            action(_icon ?? throw new ObjectDisposedException(nameof(IAvnShellTray)));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}
