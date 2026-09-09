using System;
using System.Globalization;
using Avalonia.Media;

namespace Avalonia.Host.Com;

/// <summary>
/// Converts between the ABI's UTF-16 shadow list and a
/// <see cref="BoxShadows"/>. The wire format is the same comma-separated
/// list <see cref="BoxShadows.ToString"/> prints; parsing splits on the
/// top-level commas (colors are bracket-respecting) and hands each shadow
/// to <see cref="BoxShadow.Parse"/>.
/// </summary>
internal static class AvnBoxShadows
{
    public static string ToAbi(BoxShadows value) => value.ToString();

    public static BoxShadows FromAbi(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return default;

        var text = value.Trim();
        if (text == "none")
            return default;

        var shadows = new System.Collections.Generic.List<BoxShadow>();
        var depth = 0;
        var start = 0;
        for (var i = 0; i < text.Length; i++)
        {
            var c = text[i];
            if (c == '(')
                depth++;
            else if (c == ')')
                depth--;
            else if (c == ',' && depth == 0)
            {
                AddShadow(shadows, text.AsSpan(start, i - start));
                start = i + 1;
            }
        }

        AddShadow(shadows, text.AsSpan(start));
        return shadows.Count == 0
            ? default
            : shadows.Count == 1
                ? new BoxShadows(shadows[0])
                : new BoxShadows(shadows[0], shadows.GetRange(1, shadows.Count - 1).ToArray());
    }

    private static void AddShadow(System.Collections.Generic.List<BoxShadow> shadows, ReadOnlySpan<char> token)
    {
        var trimmed = token.Trim();
        if (trimmed.Length == 0)
            return;
        shadows.Add(BoxShadow.Parse(new string(trimmed)));
    }
}
