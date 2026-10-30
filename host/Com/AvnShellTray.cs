using System;
using System.Collections.Generic;
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
    private const int KindCommand = 0;
    private const int KindCheckBox = 1;
    private const int KindSeparator = 2;
    private const int KindSubmenu = 3;
    private const int FlagChecked = 1;
    private const int FlagDisabled = 2;

    private readonly Application _application;
    private readonly Dictionary<int, NativeMenuItemBase> _items = new();
    private readonly NativeMenu _menu = new();
    private IAvnShellTrayHandler? _handler;
    private TrayIcon? _icon;

    public AvnShellTray(Application application, IAvnShellTrayHandler? handler)
    {
        _application = application;
        _handler = handler;
        _icon = new TrayIcon { Menu = _menu, IsVisible = true };
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

    public int SetToolTip(string? text) => Run(icon => icon.ToolTipText = text);

    public int SetVisible(int visible) => Run(icon => icon.IsVisible = visible != 0);

    public int ClearMenu() => Run(_ =>
    {
        _menu.Items.Clear();
        _items.Clear();
    });

    public int AddMenuItem(int parent, int id, int kind, string? header, int flags)
    {
        if (id <= 0 || kind is < KindCommand or > KindSubmenu)
            return HResults.E_INVALIDARG;
        return Run(_ =>
        {
            if (_items.ContainsKey(id))
                throw new ArgumentException($"Menu item {id} already exists.", nameof(id));
            NativeMenu target;
            if (parent == 0)
                target = _menu;
            else if (_items.TryGetValue(parent, out var owner) && owner is NativeMenuItem { Menu: { } submenu })
                target = submenu;
            else
                throw new ArgumentException($"Menu item {parent} is not a submenu.", nameof(parent));

            NativeMenuItemBase item;
            if (kind == KindSeparator)
            {
                item = new NativeMenuItemSeparator();
            }
            else
            {
                var menuItem = new NativeMenuItem(header ?? string.Empty);
                if (kind == KindSubmenu)
                    menuItem.Menu = new NativeMenu();
                else
                    menuItem.Click += (_, _) => _handler?.OnMenuItem(id);
                if (kind == KindCheckBox)
                    menuItem.ToggleType = MenuItemToggleType.CheckBox;
                Apply(menuItem, flags);
                item = menuItem;
            }
            target.Items.Add(item);
            _items.Add(id, item);
        });
    }

    public int UpdateMenuItem(int id, string? header, int flags) => Run(_ =>
    {
        if (!_items.TryGetValue(id, out var item))
            throw new ArgumentException($"Menu item {id} does not exist.", nameof(id));
        if (item is NativeMenuItem menuItem)
        {
            if (header is not null)
                menuItem.Header = header;
            Apply(menuItem, flags);
        }
    });

    private static void Apply(NativeMenuItem item, int flags)
    {
        item.IsChecked = (flags & FlagChecked) != 0;
        item.IsEnabled = (flags & FlagDisabled) == 0;
    }

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
