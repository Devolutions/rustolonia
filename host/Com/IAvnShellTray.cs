using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// A notification-area icon built from Rust (stage 34). Its menu is a tree of
/// items Rust numbers itself; a click on one is reported to the
/// <see cref="IAvnShellTrayHandler"/> by that number. UI thread only.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnShellTray)]
public partial interface IAvnShellTray
{
    /// <summary>The image: a file path or an <c>avares://</c> URI; null removes it.</summary>
    [PreserveSig]
    int SetIcon(string? source);

    [PreserveSig]
    int SetToolTip(string? text);

    [PreserveSig]
    int SetVisible(int visible);

    /// <summary>Removes every menu item.</summary>
    [PreserveSig]
    int ClearMenu();

    /// <summary>
    /// Appends item <paramref name="id"/> (positive, unique in this menu) under
    /// <paramref name="parent"/>, a submenu's id or 0 for the top level.
    /// <paramref name="kind"/>: 0 command, 1 check box, 2 separator, 3 submenu.
    /// <paramref name="flags"/>: bit 0 checked, bit 1 disabled.
    /// </summary>
    [PreserveSig]
    int AddMenuItem(int parent, int id, int kind, string? header, int flags);

    /// <summary>Changes an item's header (when not null) and flags.</summary>
    [PreserveSig]
    int UpdateMenuItem(int id, string? header, int flags);

    /// <summary>Takes the icon down for good.</summary>
    [PreserveSig]
    int Remove();
}

/// <summary>Implemented by Rust: what was clicked on a tray icon.</summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnShellTrayHandler)]
public partial interface IAvnShellTrayHandler
{
    /// <summary>The icon itself was clicked.</summary>
    [PreserveSig]
    int OnActivate();

    /// <summary>Menu item <paramref name="id"/> was clicked.</summary>
    [PreserveSig]
    int OnMenuItem(int id);
}
