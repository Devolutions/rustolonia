using Avalonia.Media;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects <see cref="TextTrimming"/> as a UTF-16 name matching
/// <see cref="TextTrimming.Parse"/>. The abstract type does not override
/// <c>ToString</c>, so conversion lives here rather than on the CLR type.
/// </summary>
internal static class AvnTextTrimming
{
    public static string ToAbi(TextTrimming value) => value.ToString() ?? nameof(TextTrimming.None);

    public static TextTrimming FromAbi(string? value) =>
        string.IsNullOrEmpty(value) ? TextTrimming.None : TextTrimming.Parse(value);
}
