using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices.Marshalling;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Rust;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>Code-first native menus (<see cref="IAvnApplication15"/>).</summary>
public partial class AvnApplication : IAvnApplication15
{
    private readonly List<AvnNativeMenu> _nativeMenus = new();
    private readonly ConditionalWeakTable<Window, NativeMenu> _windowMenuSlots = new();
    private AvnNativeMenu? _applicationMenu;

    public int CreateNativeMenu(IAvnNativeMenuHandler? handler, out IAvnNativeMenu? menu)
    {
        menu = null;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var created = new AvnNativeMenu(handler);
            _nativeMenus.Add(created);
            menu = created;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int SetApplicationMenu(IAvnNativeMenu? menu)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var application = Application.Current
                ?? throw new InvalidOperationException("An application menu needs a running application.");
            var replacement = Resolve(menu);
            if (ReferenceEquals(replacement, _applicationMenu))
                return HResults.S_OK;
            replacement?.EnsureDetachedFromWindows();
            _applicationMenu?.DetachFromApplication(application);
            _applicationMenu = null;
            if (replacement is not null)
            {
                replacement.AttachToApplication(application);
                _applicationMenu = replacement;
            }
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int SetWindowMenu(IAvnWindow? window, IAvnNativeMenu? menu)
    {
        AvnNativeMenu? replacement;
        try
        {
            replacement = Resolve(menu);
            if (replacement is not null && ReferenceEquals(replacement, _applicationMenu))
                throw new InvalidOperationException("The application menu cannot also be a window menu.");
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
        return WithWindow(window, managed =>
        {
            foreach (var owner in _nativeMenus)
                owner.DetachFromWindow(managed);
            if (replacement is not null)
            {
                replacement.AttachToWindow(managed, WindowMenuSlot(managed));
            }
            else if (_windowMenuSlots.TryGetValue(managed, out var slot)
                && !NativeMenu.GetIsNativeMenuExported(managed))
            {
                // An exported menu keeps its (now empty) slot; see WindowMenuSlot.
                _windowMenuSlots.Remove(managed);
                if (ReferenceEquals(NativeMenu.GetMenu(managed), slot))
                    NativeMenu.SetMenu(managed, null);
            }
        });
    }

    /// <summary>
    /// The menu a window shows Rust-built menus in. A window keeps one for good:
    /// the macOS exporter binds to the first menu it exports and rejects any
    /// other, so switching or removing menus changes the slot's items instead.
    /// </summary>
    private NativeMenu WindowMenuSlot(Window window)
    {
        if (!_windowMenuSlots.TryGetValue(window, out var slot))
        {
            slot = new NativeMenu();
            _windowMenuSlots.Add(window, slot);
        }
        if (!ReferenceEquals(NativeMenu.GetMenu(window), slot))
            NativeMenu.SetMenu(window, slot);
        return slot;
    }

    public int IsNativeMenuExported(IAvnWindow? window, out int exported)
    {
        var value = 0;
        var hr = WithWindow(window, managed => value = NativeMenu.GetIsNativeMenuExported(managed) ? 1 : 0);
        exported = value;
        return hr;
    }

    public int CreateNativeMenuBar(out IAvnControl? bar)
    {
        bar = null;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            bar = (IAvnControl?)ProjectionRuntime.Wrap(new NativeMenuBar());
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    /// <summary>
    /// Detaches every Rust-built menu and drops its handler, so no Rust callback
    /// runs while the runtime shuts down.
    /// </summary>
    private void ReleaseNativeMenus()
    {
        var application = Application.Current;
        foreach (var menu in _nativeMenus)
        {
            try
            {
                if (application is not null && ReferenceEquals(menu, _applicationMenu))
                    menu.DetachFromApplication(application);
                menu.Dispose();
            }
            catch (Exception)
            {
                // Shutdown keeps going; the platform drops the menu with the process.
            }
        }
        _applicationMenu = null;
        _nativeMenus.Clear();
    }

    private static AvnNativeMenu? Resolve(IAvnNativeMenu? menu) => menu switch
    {
        null => null,
        AvnNativeMenu ours => ours,
        _ => throw new ArgumentException("The menu was not created by this host.", nameof(menu)),
    };
}

/// <summary>
/// A <see cref="NativeMenu"/> Rust builds item by item, attached to windows
/// (with its shortcuts as key bindings) or spliced into the application menu.
/// </summary>
[GeneratedComClass]
internal sealed partial class AvnNativeMenu : IAvnNativeMenu, IDisposable
{
    private const string DefaultAboutHeader = "About Avalonia";

    private readonly NativeMenuTree _tree;
    private readonly Dictionary<Window, WindowState> _windows = new();
    private IAvnNativeMenuHandler? _handler;
    private bool _setApplicationProperty;
    private (NativeMenu Host, int Index, NativeMenuItemBase Item)? _removedAbout;

    public AvnNativeMenu(IAvnNativeMenuHandler? handler)
    {
        _handler = handler;
        _tree = new NativeMenuTree(Invoke);
        _tree.Changed += RefreshKeyBindings;
    }

    public NativeMenu Menu => _tree.Menu;

    public int ClearMenu() => Run(() => _tree.Clear());

    public int AddMenuItem(int parent, int id, int kind, string? header, string? gesture, int flags) =>
        Run(() => _tree.Add(parent, id, kind, header, gesture, flags));

    public int UpdateMenuItem(int id, string? header, int flags) => Run(() => _tree.Update(id, header, flags));

    /// <summary>Shows a copy of the items in <paramref name="slot"/>, the window's menu.</summary>
    public void AttachToWindow(Window window, NativeMenu slot)
    {
        if (_windows.ContainsKey(window))
            return;
        _tree.AddView(slot);
        _windows.Add(window, new WindowState(slot, new List<KeyBinding>()));
        window.Closed += OnWindowClosed;
        InstallKeyBindings(window);
    }

    public void DetachFromWindow(Window window)
    {
        if (!_windows.TryGetValue(window, out var state))
            return;
        RemoveKeyBindings(window);
        _tree.RemoveView(state.Slot);
        _windows.Remove(window);
        window.Closed -= OnWindowClosed;
    }

    public void EnsureDetachedFromWindows()
    {
        if (_windows.Count != 0)
            throw new InvalidOperationException("A window menu cannot also be the application menu.");
    }

    /// <summary>
    /// Puts the items at the top of the application menu. Where the platform
    /// already exports one (macOS creates it, with "About Avalonia" and the
    /// standard Services/Hide/Quit items, before Rust runs), the items join it,
    /// replacing the default About item; Avalonia does not re-export a menu
    /// set on the application later.
    /// </summary>
    public void AttachToApplication(Application application)
    {
        var existing = NativeMenu.GetMenu(application);
        if (existing is null)
        {
            NativeMenu.SetMenu(application, _tree.Menu);
            _setApplicationProperty = true;
            return;
        }
        if (ReferenceEquals(existing, _tree.Menu))
            return;
        if (existing.Items.Count > 0 && existing.Items[0] is NativeMenuItem { Header: DefaultAboutHeader } about)
        {
            existing.Items.RemoveAt(0);
            _removedAbout = (existing, 0, about);
        }
        _tree.Rehost(existing);
    }

    public void DetachFromApplication(Application application)
    {
        var host = _tree.RootHost;
        _tree.Rehost(null);
        if (_removedAbout is { } removed)
        {
            _removedAbout = null;
            if (ReferenceEquals(removed.Host, host) && removed.Item.Parent is null)
                host.Items.Insert(Math.Min(removed.Index, host.Items.Count), removed.Item);
        }
        if (_setApplicationProperty)
        {
            _setApplicationProperty = false;
            if (ReferenceEquals(NativeMenu.GetMenu(application), _tree.Menu))
                NativeMenu.SetMenu(application, null);
        }
    }

    public void Dispose()
    {
        _handler = null;
        foreach (var window in new List<Window>(_windows.Keys))
            DetachFromWindow(window);
    }

    private void Invoke(int id)
    {
        if (_handler is { } handler)
            handler.OnMenuItem(id);
    }

    private void OnWindowClosed(object? sender, EventArgs e)
    {
        if (sender is Window window)
            DetachFromWindow(window);
    }

    /// <summary>
    /// Only a real native menu bar handles its own accelerators; everywhere
    /// else <see cref="NativeMenuBar"/> shows the gesture without handling it.
    /// Like the schema-built menus, shortcuts are therefore always installed
    /// as window key bindings too.
    /// </summary>
    private void InstallKeyBindings(Window window)
    {
        var bindings = _windows[window].Bindings;
        foreach (var (id, gesture) in _tree.Gestures())
        {
            var binding = new KeyBinding { Gesture = gesture, Command = new ItemCommand(this, id) };
            bindings.Add(binding);
            window.KeyBindings.Add(binding);
        }
    }

    private void RemoveKeyBindings(Window window)
    {
        var bindings = _windows[window].Bindings;
        foreach (var binding in bindings)
            window.KeyBindings.Remove(binding);
        bindings.Clear();
    }

    private void RefreshKeyBindings()
    {
        foreach (var window in _windows.Keys)
        {
            RemoveKeyBindings(window);
            InstallKeyBindings(window);
        }
    }

    private int Run(Action action)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            action();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    private sealed record WindowState(NativeMenu Slot, List<KeyBinding> Bindings);

    private sealed class ItemCommand(AvnNativeMenu owner, int id) : ICommand
    {
        public event EventHandler? CanExecuteChanged { add { } remove { } }

        public bool CanExecute(object? parameter) => owner._tree.IsInvokable(id);

        public void Execute(object? parameter)
        {
            if (owner._tree.IsInvokable(id))
                owner.Invoke(id);
        }
    }
}
