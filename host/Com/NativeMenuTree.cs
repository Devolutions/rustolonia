using System;
using System.Collections.Generic;
using Avalonia.Controls;
using Avalonia.Input;

namespace Avalonia.Host.Com;

/// <summary>
/// A <see cref="NativeMenu"/> Rust builds item by item, numbering the items
/// itself. Shared by Rust-built tray icons (stage 34) and Rust-built window and
/// application menus (<see cref="IAvnApplication.SetApplicationMenu"/>). UI thread only.
/// </summary>
/// <remarks>
/// <para>
/// Clicking a check box or radio item does not change it: the click is
/// reported and the application writes the new state back, so Rust stays the
/// owner of every check mark.
/// </para>
/// <para>
/// An Avalonia menu item has a single parent, so a menu shown in several
/// places is built once per place: <see cref="Menu"/> is the primary copy and
/// <see cref="AddView"/> mirrors every item into another menu, kept in step
/// with every later change.
/// </para>
/// </remarks>
internal sealed class NativeMenuTree
{
    public const int KindCommand = 0;
    public const int KindCheckBox = 1;
    public const int KindSeparator = 2;
    public const int KindSubmenu = 3;
    public const int KindRadio = 4;
    public const int FlagChecked = 1;
    public const int FlagDisabled = 2;
    public const int FlagHidden = 4;

    private readonly Dictionary<int, Spec> _specs = new();
    private readonly List<Spec> _order = new();
    private readonly List<View> _views = new();
    private readonly View _primary;
    private readonly Action<int> _clicked;

    public NativeMenuTree(Action<int> clicked)
    {
        _clicked = clicked;
        _primary = new View(this, Menu);
        _views.Add(_primary);
    }

    /// <summary>The primary copy's own menu, where its top-level items live unless rehosted.</summary>
    public NativeMenu Menu { get; } = new();

    /// <summary>The menu currently holding the primary copy's top-level items.</summary>
    public NativeMenu RootHost => _primary.Host;

    /// <summary>Raised after any change that can affect gestures or enabled state.</summary>
    public event Action? Changed;

    public void Clear()
    {
        foreach (var view in _views)
            view.Clear();
        _specs.Clear();
        _order.Clear();
        Changed?.Invoke();
    }

    public void Add(int parent, int id, int kind, string? header, string? gesture, int flags)
    {
        if (id <= 0)
            throw new ArgumentException("Menu item ids must be positive.", nameof(id));
        if (kind is < KindCommand or > KindRadio)
            throw new ArgumentException("Unknown menu item kind.", nameof(kind));
        if (_specs.ContainsKey(id))
            throw new ArgumentException($"Menu item {id} already exists.", nameof(id));
        if (parent != 0 && (!_specs.TryGetValue(parent, out var owner) || owner.Kind != KindSubmenu))
            throw new ArgumentException($"Menu item {parent} is not a submenu.", nameof(parent));

        var parsedGesture = string.IsNullOrWhiteSpace(gesture) ? null : KeyGesture.Parse(gesture);
        if (parsedGesture is not null && kind is KindSeparator or KindSubmenu)
            throw new ArgumentException("Separators and submenus cannot have a gesture.", nameof(gesture));

        var spec = new Spec(id, parent, kind, parsedGesture) { Header = header ?? string.Empty, Flags = flags };
        _specs.Add(id, spec);
        _order.Add(spec);
        foreach (var view in _views)
            view.Build(spec);
        Changed?.Invoke();
    }

    public void Update(int id, string? header, int flags)
    {
        if (!_specs.TryGetValue(id, out var spec))
            throw new ArgumentException($"Menu item {id} does not exist.", nameof(id));
        if (header is not null)
            spec.Header = header;
        spec.Flags = flags;
        foreach (var view in _views)
            view.Refresh(spec);
        Changed?.Invoke();
    }

    /// <summary>
    /// Moves the primary copy's top-level items into <paramref name="host"/>
    /// (at its start), or back into <see cref="Menu"/> when it is null. Used to
    /// place an application's items in the menu the platform already exports.
    /// </summary>
    public void Rehost(NativeMenu? host)
    {
        _primary.Rehost(host ?? Menu);
        Changed?.Invoke();
    }

    /// <summary>Builds another copy of every item at the start of <paramref name="host"/>.</summary>
    public void AddView(NativeMenu host)
    {
        if (_views.Exists(v => ReferenceEquals(v.Host, host)))
            throw new InvalidOperationException("The menu already shows these items.");
        var view = new View(this, host);
        foreach (var spec in _order)
            view.Build(spec);
        _views.Add(view);
    }

    /// <summary>Removes the copy <see cref="AddView"/> built in <paramref name="host"/>.</summary>
    public void RemoveView(NativeMenu host)
    {
        var index = _views.FindIndex(v => !ReferenceEquals(v, _primary) && ReferenceEquals(v.Host, host));
        if (index < 0)
            return;
        _views[index].Clear();
        _views.RemoveAt(index);
    }

    /// <summary>Every item id carrying a gesture, with the gesture.</summary>
    public IEnumerable<(int Id, KeyGesture Gesture)> Gestures()
    {
        foreach (var spec in _order)
        {
            if (spec.Gesture is { } gesture)
                yield return (spec.Id, gesture);
        }
    }

    /// <summary>
    /// Whether item <paramref name="id"/> can be invoked right now: it and every
    /// submenu above it are enabled and visible.
    /// </summary>
    public bool IsInvokable(int id)
    {
        while (id != 0)
        {
            if (!_specs.TryGetValue(id, out var spec) || spec.Kind == KindSeparator
                || (spec.Flags & (FlagDisabled | FlagHidden)) != 0)
                return false;
            id = spec.Parent;
        }
        return true;
    }

    private sealed class Spec(int id, int parent, int kind, KeyGesture? gesture)
    {
        public int Id { get; } = id;
        public int Parent { get; } = parent;
        public int Kind { get; } = kind;
        public KeyGesture? Gesture { get; } = gesture;
        public string Header { get; set; } = string.Empty;
        public int Flags { get; set; }
    }

    /// <summary>One copy of the items, with its top-level items at the start of <see cref="Host"/>.</summary>
    private sealed class View(NativeMenuTree tree, NativeMenu host)
    {
        private readonly Dictionary<int, NativeMenuItemBase> _items = new();
        private readonly List<NativeMenuItemBase> _roots = new();

        public NativeMenu Host { get; private set; } = host;

        public void Build(Spec spec)
        {
            NativeMenuItemBase item;
            if (spec.Kind == KindSeparator)
            {
                item = new NativeMenuItemSeparator();
            }
            else
            {
                var menuItem = new NativeMenuItem(spec.Header) { Gesture = spec.Gesture };
                if (spec.Kind == KindSubmenu)
                {
                    menuItem.Menu = new NativeMenu();
                }
                else
                {
                    var id = spec.Id;
                    menuItem.Click += (_, _) => tree._clicked(id);
                }
                menuItem.ToggleType = spec.Kind switch
                {
                    KindCheckBox => MenuItemToggleType.CheckBox,
                    KindRadio => MenuItemToggleType.Radio,
                    _ => MenuItemToggleType.None,
                };
                item = menuItem;
            }
            Apply(item, spec.Flags);

            if (spec.Parent == 0)
            {
                Host.Items.Insert(RootInsertIndex(), item);
                _roots.Add(item);
            }
            else
            {
                ((NativeMenuItem)_items[spec.Parent]).Menu!.Items.Add(item);
            }
            _items.Add(spec.Id, item);
        }

        public void Refresh(Spec spec)
        {
            var item = _items[spec.Id];
            if (item is NativeMenuItem menuItem and not NativeMenuItemSeparator)
                menuItem.Header = spec.Header;
            Apply(item, spec.Flags);
        }

        public void Clear()
        {
            foreach (var root in _roots)
                Host.Items.Remove(root);
            _roots.Clear();
            _items.Clear();
        }

        public void Rehost(NativeMenu host)
        {
            if (ReferenceEquals(host, Host))
                return;
            foreach (var root in _roots)
                Host.Items.Remove(root);
            Host = host;
            for (var i = 0; i < _roots.Count; i++)
                host.Items.Insert(i, _roots[i]);
        }

        private int RootInsertIndex() =>
            _roots.Count == 0 ? 0 : Host.Items.IndexOf(_roots[^1]) + 1;

        private static void Apply(NativeMenuItemBase item, int flags)
        {
            if (item is not NativeMenuItem menuItem)
                return;
            menuItem.IsVisible = (flags & FlagHidden) == 0;
            if (menuItem is NativeMenuItemSeparator)
                return;
            menuItem.IsChecked = (flags & FlagChecked) != 0;
            menuItem.IsEnabled = (flags & FlagDisabled) == 0;
        }
    }
}
