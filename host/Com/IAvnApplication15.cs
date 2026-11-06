using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Code-first native menus: Avalonia's <c>NativeMenu</c> built from Rust.
/// Queried as an optional interface from <see cref="IAvnApplication"/>; no
/// published vtable changes. UI thread only.
/// </summary>
/// <remarks>
/// A window menu is exported to the platform menu bar where there is one
/// (macOS) and drawn in the window by a <see cref="CreateNativeMenuBar"/>
/// control everywhere else; the control hides itself when the menu is
/// exported, so an application places one in every window unconditionally.
/// The application menu is the macOS application-named menu (About,
/// Settings…); other platforms do not show it.
/// </remarks>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnApplication15)]
public partial interface IAvnApplication15
{
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
}

/// <summary>
/// A native menu Rust builds item by item, numbering the items itself. A click
/// is reported to the <see cref="IAvnNativeMenuHandler"/> by that number.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnNativeMenu)]
public partial interface IAvnNativeMenu
{
    /// <summary>Removes every item.</summary>
    [PreserveSig]
    int ClearMenu();

    /// <summary>
    /// Appends item <paramref name="id"/> (positive, unique in this menu) under
    /// <paramref name="parent"/>, a submenu's id or 0 for the top level.
    /// <paramref name="kind"/>: 0 command, 1 check box, 2 separator,
    /// 3 submenu, 4 radio. <paramref name="gesture"/> is a key gesture such as
    /// <c>Ctrl+O</c> or <c>Cmd+Shift+S</c>, or null.
    /// <paramref name="flags"/>: bit 0 checked, bit 1 disabled, bit 2 hidden.
    /// </summary>
    [PreserveSig]
    int AddMenuItem(int parent, int id, int kind, string? header, string? gesture, int flags);

    /// <summary>Changes an item's header (when not null) and flags.</summary>
    [PreserveSig]
    int UpdateMenuItem(int id, string? header, int flags);
}

/// <summary>Implemented by Rust: what was clicked on a native menu.</summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnNativeMenuHandler)]
public partial interface IAvnNativeMenuHandler
{
    /// <summary>
    /// Menu item <paramref name="id"/> was clicked or its shortcut pressed.
    /// Check box and radio items are not toggled: the application updates them.
    /// </summary>
    [PreserveSig]
    int OnMenuItem(int id);
}
