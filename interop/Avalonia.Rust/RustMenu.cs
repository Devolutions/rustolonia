using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Input;

namespace Avalonia.Rust;

/// <summary>
/// Lifetime scope for one built menu: every property observer, collection
/// observer and attached top-level state a generated menu factory creates is
/// registered here and released together.
/// </summary>
/// <remarks>
/// Menus are presentation, so nothing in this file crosses the ABI. A generated
/// factory reads already generated commands and properties off a typed adapter
/// and writes Avalonia menu objects; there is no reflection, which keeps the
/// whole path NativeAOT-safe.
/// </remarks>
public sealed class RustMenuScope : IDisposable
{
    private readonly Dictionary<string, List<Action>> _observers = new(StringComparer.Ordinal);
    private readonly List<Action> _cleanup = new();
    private INotifyPropertyChanged? _source;
    private bool _disposed;

    public RustMenuScope(INotifyPropertyChanged source)
    {
        ArgumentNullException.ThrowIfNull(source);
        _source = source;
        source.PropertyChanged += OnPropertyChanged;
    }

    /// <summary>Whether this scope has been disposed.</summary>
    public bool IsDisposed => _disposed;

    /// <summary>
    /// Applies <paramref name="apply"/> now and again whenever
    /// <paramref name="propertyName"/> changes on the model.
    /// </summary>
    public void Observe(string propertyName, Action apply)
    {
        ArgumentException.ThrowIfNullOrEmpty(propertyName);
        ArgumentNullException.ThrowIfNull(apply);
        if (_disposed)
            return;
        if (!_observers.TryGetValue(propertyName, out var list))
            _observers[propertyName] = list = new List<Action>();
        list.Add(apply);
        apply();
    }

    /// <summary>
    /// Applies <paramref name="apply"/> now and again whenever the collection
    /// changes. Recent-file submenus are the only dynamic menu content, and
    /// they are rebuilt wholesale because an MRU list is short by construction.
    /// </summary>
    public void ObserveCollection(INotifyCollectionChanged collection, Action apply)
    {
        ArgumentNullException.ThrowIfNull(collection);
        ArgumentNullException.ThrowIfNull(apply);
        if (_disposed)
            return;
        void Handler(object? sender, NotifyCollectionChangedEventArgs e) => apply();
        collection.CollectionChanged += Handler;
        _cleanup.Add(() => collection.CollectionChanged -= Handler);
        apply();
    }

    /// <summary>Registers an arbitrary teardown action run on dispose.</summary>
    public void OnDispose(Action cleanup)
    {
        ArgumentNullException.ThrowIfNull(cleanup);
        if (_disposed)
        {
            cleanup();
            return;
        }

        _cleanup.Add(cleanup);
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;
        if (_source is { } source)
            source.PropertyChanged -= OnPropertyChanged;
        _source = null;
        _observers.Clear();
        for (var i = _cleanup.Count - 1; i >= 0; i--)
        {
            try { _cleanup[i](); }
            catch (Exception) { /* teardown is best effort */ }
        }

        _cleanup.Clear();
    }

    private void OnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (_disposed)
            return;
        if (e.PropertyName is null)
        {
            foreach (var list in _observers.Values)
                Invoke(list);
            return;
        }

        if (_observers.TryGetValue(e.PropertyName, out var observers))
            Invoke(observers);
    }

    private static void Invoke(List<Action> observers)
    {
        // Snapshotted because an observer may write back to the model, and a
        // write-back raises PropertyChanged re-entrantly.
        for (var i = 0; i < observers.Count; i++)
            observers[i]();
    }
}

/// <summary>
/// The command behind one generated menu item or accelerator.
/// </summary>
/// <remarks>
/// A menu item's enabled state is not set directly: both
/// <see cref="NativeMenuItem"/> and <see cref="MenuItem"/> derive
/// <c>IsEnabled</c> from their command's <see cref="ICommand.CanExecute"/> and
/// overwrite any local value when it changes. Composing the generated command's
/// own <c>CanExecute</c> with a declared Boolean property therefore has to
/// happen here, in one command, rather than by assigning <c>IsEnabled</c>.
/// </remarks>
public sealed class RustMenuCommand : ICommand
{
    private readonly Action<object?> _execute;
    private readonly Func<bool>? _canExecute;

    public RustMenuCommand(Action<object?> execute, Func<bool>? canExecute = null)
    {
        ArgumentNullException.ThrowIfNull(execute);
        _execute = execute;
        _canExecute = canExecute;
    }

    public event EventHandler? CanExecuteChanged;

    public bool CanExecute(object? parameter) => _canExecute?.Invoke() ?? true;

    public void Execute(object? parameter)
    {
        if (CanExecute(parameter))
            _execute(parameter);
    }

    public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);

    /// <summary>
    /// Forwards <paramref name="inner"/>'s <see cref="ICommand.CanExecuteChanged"/>,
    /// so a Rust-driven <c>SetCommandEnabled</c> still reaches the menu item.
    /// </summary>
    public void TrackSource(ICommand inner, RustMenuScope scope)
    {
        ArgumentNullException.ThrowIfNull(inner);
        ArgumentNullException.ThrowIfNull(scope);
        void Handler(object? sender, EventArgs e) => RaiseCanExecuteChanged();
        inner.CanExecuteChanged += Handler;
        scope.OnDispose(() => inner.CanExecuteChanged -= Handler);
    }
}

/// <summary>
/// Helpers shared by every generated menu factory.
/// </summary>
public static class RustMenu
{    /// <summary>
    /// Parses a declared accelerator such as <c>Ctrl+O</c>. Returns null for a
    /// null or blank declaration; an unparsable gesture is a schema error and
    /// throws, because silently dropping a shortcut is worse than failing at
    /// window construction.
    /// </summary>
    public static KeyGesture? ParseGesture(string? gesture) =>
        string.IsNullOrWhiteSpace(gesture) ? null : KeyGesture.Parse(gesture);

    /// <summary>
    /// Derives a readable menu header from a storage URI.
    /// </summary>
    /// <remarks>
    /// A stage 29 storage item only guarantees a URI, so the recent-file list
    /// stores URIs and the label is derived here rather than transported. The
    /// last non-empty, unescaped path segment is used, falling back to the whole
    /// URI when there is none.
    /// </remarks>
    public static string RecentFileHeader(string? uri)
    {
        if (string.IsNullOrWhiteSpace(uri))
            return "";
        if (Uri.TryCreate(uri, UriKind.Absolute, out var parsed))
        {
            var segments = parsed.Segments;
            for (var i = segments.Length - 1; i >= 0; i--)
            {
                var segment = Uri.UnescapeDataString(segments[i]).TrimEnd('/');
                if (!string.IsNullOrEmpty(segment))
                    return segment;
            }

            return parsed.AbsoluteUri;
        }

        var separator = uri.LastIndexOfAny(['/', '\\']);
        return separator >= 0 && separator < uri.Length - 1 ? uri[(separator + 1)..] : uri;
    }

    /// <summary>
    /// Rebuilds <paramref name="items"/> from the recent-file URIs, one
    /// <see cref="NativeMenuItem"/> per entry, plus an
    /// <paramref name="emptyHeader"/> placeholder when the list is empty so the
    /// submenu is never an empty popup.
    /// </summary>
    public static void FillRecentFiles(
        IList<NativeMenuItemBase> items,
        IEnumerable uris,
        ICommand activate,
        string emptyHeader,
        int capacity)
    {
        ArgumentNullException.ThrowIfNull(items);
        ArgumentNullException.ThrowIfNull(uris);
        ArgumentNullException.ThrowIfNull(activate);
        items.Clear();
        var count = 0;
        foreach (var value in uris)
        {
            if (count >= capacity)
                break;
            if (value is not string uri || string.IsNullOrWhiteSpace(uri))
                continue;
            items.Add(new NativeMenuItem(RecentFileHeader(uri))
            {
                Command = activate,
                CommandParameter = uri,
                ToolTip = uri,
            });
            count++;
        }

        if (count == 0)
            items.Add(new NativeMenuItem(emptyHeader) { IsEnabled = false });
    }

    /// <summary>
    /// Rebuilds a managed <see cref="MenuItem"/> submenu from the recent-file
    /// URIs. Used by generated context menus, which are ordinary controls.
    /// </summary>
    public static void FillRecentFiles(
        MenuItem parent,
        IEnumerable uris,
        ICommand activate,
        string emptyHeader,
        int capacity)
    {
        ArgumentNullException.ThrowIfNull(parent);
        ArgumentNullException.ThrowIfNull(uris);
        ArgumentNullException.ThrowIfNull(activate);
        var items = new List<Control>();
        foreach (var value in uris)
        {
            if (items.Count >= capacity)
                break;
            if (value is not string uri || string.IsNullOrWhiteSpace(uri))
                continue;
            items.Add(new MenuItem
            {
                Header = RecentFileHeader(uri),
                Command = activate,
                CommandParameter = uri,
            });
        }

        if (items.Count == 0)
            items.Add(new MenuItem { Header = emptyHeader, IsEnabled = false });
        parent.ItemsSource = items;
    }

    /// <summary>
    /// Installs <paramref name="bindings"/> on <paramref name="target"/> and
    /// registers their removal with <paramref name="scope"/>.
    /// </summary>
    /// <remarks>
    /// Only a real native menu bar (macOS today) handles its own accelerators.
    /// Everywhere else <c>NativeMenuBar</c> renders the same model in-window and
    /// shows the gesture as a label without handling it, so a declared shortcut
    /// is always additionally installed as an input-element key binding. That is
    /// also what makes an accelerator work when it is not on any menu at all.
    /// </remarks>
    public static void AttachKeyBindings(
        InputElement target,
        IReadOnlyList<KeyBinding> bindings,
        RustMenuScope scope)
    {
        ArgumentNullException.ThrowIfNull(target);
        ArgumentNullException.ThrowIfNull(bindings);
        ArgumentNullException.ThrowIfNull(scope);
        foreach (var binding in bindings)
            target.KeyBindings.Add(binding);
        scope.OnDispose(() =>
        {
            foreach (var binding in bindings)
                target.KeyBindings.Remove(binding);
        });
    }
}

/// <summary>
/// A menu attached to a top-level: the built model, its accelerators and every
/// observer wired to keep them current. Disposing detaches all of it.
/// </summary>
public sealed class RustMenuAttachment : IDisposable
{
    private readonly RustMenuScope _scope;
    private TopLevel? _target;

    internal RustMenuAttachment(TopLevel target, NativeMenu menu, IReadOnlyList<KeyBinding> keyBindings, RustMenuScope scope)
    {
        _target = target;
        _scope = scope;
        Menu = menu;
        KeyBindings = keyBindings;
    }

    /// <summary>The built application menu.</summary>
    public NativeMenu Menu { get; }

    /// <summary>The accelerators installed on the top-level.</summary>
    public IReadOnlyList<KeyBinding> KeyBindings { get; }

    /// <summary>
    /// Attaches <paramref name="menu"/> and <paramref name="keyBindings"/> to a
    /// top-level. The menu is exported natively where the platform has a menu
    /// bar and rendered by a <c>NativeMenuBar</c> control otherwise; both read
    /// the same attached <c>NativeMenu.Menu</c> value, so there is no platform
    /// branch here.
    /// </summary>
    public static RustMenuAttachment Attach(
        TopLevel target,
        NativeMenu menu,
        IReadOnlyList<KeyBinding> keyBindings,
        RustMenuScope scope)
    {
        ArgumentNullException.ThrowIfNull(target);
        ArgumentNullException.ThrowIfNull(menu);
        ArgumentNullException.ThrowIfNull(keyBindings);
        ArgumentNullException.ThrowIfNull(scope);
        NativeMenu.SetMenu(target, menu);
        RustMenu.AttachKeyBindings(target, keyBindings, scope);
        return new RustMenuAttachment(target, menu, keyBindings, scope);
    }

    public void Dispose()
    {
        if (_target is { } target)
        {
            if (ReferenceEquals(NativeMenu.GetMenu(target), Menu))
                NativeMenu.SetMenu(target, null);
            _target = null;
        }

        _scope.Dispose();
    }
}
