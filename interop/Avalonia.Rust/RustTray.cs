using System;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Platform;

namespace Avalonia.Rust;

/// <summary>
/// A notification-area icon built from a generated tray menu: the icon, its
/// tooltip, the menu and every observer that keeps them current. Disposing
/// removes the icon and releases the menu scope and the model adapter.
/// </summary>
/// <remarks>
/// Avalonia only shows a <see cref="TrayIcon"/> that is in the application's
/// <see cref="TrayIcons"/> collection, so attaching adds it there (creating the
/// collection on first use) and disposing removes it. Avalonia itself disposes
/// whatever is left in the collection when the dispatcher shuts down.
/// </remarks>
public sealed class RustTrayAttachment : IDisposable
{
    private readonly RustMenuScope _scope;
    private readonly IDisposable? _owner;
    private TrayIcon? _icon;

    private RustTrayAttachment(TrayIcon icon, RustMenuScope scope, IDisposable? owner)
    {
        _icon = icon;
        _scope = scope;
        _owner = owner;
    }

    /// <summary>The attached icon, until disposed.</summary>
    public TrayIcon? Icon => _icon;

    /// <summary>
    /// Shows <paramref name="menu"/> as the menu of a new tray icon.
    /// </summary>
    /// <param name="menu">The generated tray menu.</param>
    /// <param name="iconSource">An <c>avares://</c> resource URI or a file path.</param>
    /// <param name="toolTip">Hover text; none when null.</param>
    /// <param name="activate">Run when the icon itself is clicked; none when null.</param>
    /// <param name="scope">The menu's observer scope, released on dispose.</param>
    /// <param name="owner">Disposed last, typically the model adapter the menu reads.</param>
    public static RustTrayAttachment Attach(
        NativeMenu menu,
        string iconSource,
        string? toolTip,
        ICommand? activate,
        RustMenuScope scope,
        IDisposable? owner)
    {
        ArgumentNullException.ThrowIfNull(menu);
        ArgumentException.ThrowIfNullOrWhiteSpace(iconSource);
        ArgumentNullException.ThrowIfNull(scope);
        var application = Application.Current
            ?? throw new InvalidOperationException("A tray icon needs a running application.");
        var icon = new TrayIcon
        {
            Icon = LoadIcon(iconSource),
            ToolTipText = toolTip,
            Menu = menu,
            Command = activate,
            IsVisible = true,
        };
        var icons = TrayIcon.GetIcons(application);
        if (icons is null)
        {
            icons = new TrayIcons();
            TrayIcon.SetIcons(application, icons);
        }
        icons.Add(icon);
        return new RustTrayAttachment(icon, scope, owner);
    }

    /// <summary>Loads an icon from an <c>avares://</c> URI or a file path.</summary>
    public static WindowIcon LoadIcon(string source)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(source);
        if (source.StartsWith("avares://", StringComparison.OrdinalIgnoreCase))
        {
            using var stream = AssetLoader.Open(new Uri(source));
            return new WindowIcon(stream);
        }
        return new WindowIcon(source);
    }

    public void Dispose()
    {
        if (_icon is { } icon)
        {
            _icon = null;
            if (Application.Current is { } application && TrayIcon.GetIcons(application) is { } icons)
                icons.Remove(icon);
            icon.Dispose();
        }
        _scope.Dispose();
        _owner?.Dispose();
    }
}
