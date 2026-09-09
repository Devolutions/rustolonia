using Avalonia.Controls;

namespace Avalonia.Host.Com;

/// <summary>
/// Converts a window/tray icon between the ABI's UTF-16 file path and the
/// managed <see cref="WindowIcon"/>. The ABI slot is write-oriented: an icon
/// is loaded from a path, and reading back yields null because a
/// <see cref="WindowIcon"/> carries no source path to return.
/// </summary>
public static class AvnWindowIcon
{
    public static WindowIcon? FromAbi(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;
        try
        {
            return new WindowIcon(value);
        }
        catch
        {
            return null;
        }
    }

    public static string? ToAbi(WindowIcon? value) => null;
}
