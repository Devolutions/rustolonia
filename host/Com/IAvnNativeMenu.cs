using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

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
