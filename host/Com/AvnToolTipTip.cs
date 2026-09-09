namespace Avalonia.Host.Com;

/// <summary>
/// Projects the <c>ToolTip.Tip</c> attached property as a string.
/// </summary>
/// <remarks>
/// The managed property is an <c>object</c> so that XAML can hang a whole control off it. Over
/// the ABI it is text and nothing else: writing sets the string, and reading returns the string
/// only when the tip actually is one. A tip that is a control — set from XAML, or by a later wave
/// that projects control tips — reads back as <c>null</c> rather than as its type name.
/// </remarks>
internal static class AvnToolTipTip
{
    /// <summary>The tip text, or <c>null</c> when the control has no tip or a non-text one.</summary>
    public static string? ToAbi(object? value) => value as string;

    /// <summary>The tip to store; an empty string clears the tip.</summary>
    public static object? FromAbi(string? value) => string.IsNullOrEmpty(value) ? null : value;
}
