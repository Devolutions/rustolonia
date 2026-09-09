using System;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects a nullable <see cref="Uri"/> — <c>HyperlinkButton.NavigateUri</c> — as a UTF-16
/// string, because the projection has no URI ABI shape of its own.
/// </summary>
/// <remarks>
/// Reading uses <see cref="Uri.OriginalString"/> so a relative URI written as
/// <c>docs/readme.md</c> reads back as that spelling rather than as a <c>file://</c> absolute.
/// Writing accepts any spelling <see cref="Uri.TryCreate(string, UriKind, out Uri)"/> takes as
/// relative-or-absolute. A null or empty string clears the property. A malformed URI fails the
/// call with <see cref="UriFormatException"/> rather than being stored as a string the launcher
/// cannot open.
/// </remarks>
internal static class AvnUri
{
    public static string? ToAbi(Uri? value) => value?.OriginalString;

    public static Uri? FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
            return null;
        if (Uri.TryCreate(value, UriKind.RelativeOrAbsolute, out var uri))
            return uri;
        throw new UriFormatException($"'{value}' is not a URI.");
    }
}
