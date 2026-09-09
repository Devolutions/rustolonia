using System;
using System.Globalization;
using Avalonia.Controls;

namespace Avalonia.Host.Com;

/// <summary>
/// Converts between the ABI's UTF-16 dock name and a nullable
/// <see cref="Dock"/>. A null or empty string maps to the unset
/// placement; anything else must name a dock value.
/// </summary>
internal static class AvnDock
{
    public static string? ToAbi(Dock? value) =>
        value is null ? null : value.Value.ToString();

    public static Dock? FromAbi(string? value) =>
        value switch
        {
            null or "" => null,
            nameof(Dock.Left) => Dock.Left,
            nameof(Dock.Top) => Dock.Top,
            nameof(Dock.Right) => Dock.Right,
            nameof(Dock.Bottom) => Dock.Bottom,
            _ => throw new FormatException($"'{value}' is not a dock"),
        };
}
