using System;
using Avalonia.Styling;

namespace Avalonia.Host.Com;

/// <summary>
/// Converts between the ABI's UTF-16 theme-variant name and a
/// <see cref="ThemeVariant"/>. Only the three well-known variants cross;
/// a null or empty string maps to an unset (null) variant, and
/// <see cref="ThemeVariant.Default"/> maps to the "Default" name.
/// </summary>
internal static class AvnThemeVariant
{
    public static string? ToAbi(ThemeVariant? value) =>
        value is null ? null : value.ToString();

    public static ThemeVariant? FromAbi(string? value) =>
        value switch
        {
            null or "" => null,
            nameof(ThemeVariant.Default) => ThemeVariant.Default,
            nameof(ThemeVariant.Light) => ThemeVariant.Light,
            nameof(ThemeVariant.Dark) => ThemeVariant.Dark,
            _ => throw new FormatException($"'{value}' is not a theme variant"),
        };
}
