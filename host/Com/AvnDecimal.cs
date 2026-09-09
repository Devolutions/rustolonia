using System;
using System.Globalization;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects a nullable <see cref="decimal"/> — <c>NumericUpDown.Value</c> — as an invariant
/// UTF-16 string, because the projection has no decimal ABI shape of its own.
/// </summary>
/// <remarks>
/// Reading uses the invariant general format, so <c>1.5</c> reads back as <c>1.5</c> rather than
/// as a locale spelling. Writing accepts any spelling
/// <see cref="decimal.Parse(string, NumberStyles, IFormatProvider)"/> takes under
/// <see cref="NumberStyles.Number"/> and <see cref="CultureInfo.InvariantCulture"/>. A null or
/// empty string clears the value. A locale spelling such as <c>1,5</c> is rejected.
/// </remarks>
internal static class AvnDecimal
{
    public static string? ToAbi(decimal? value) =>
        value?.ToString(CultureInfo.InvariantCulture);

    public static decimal? FromAbi(string? value) =>
        string.IsNullOrEmpty(value) ? null : AvnDecimalValue.FromAbi(value);
}

/// <summary>
/// Projects a non-nullable <see cref="decimal"/> — <c>NumericUpDown.Minimum</c>,
/// <c>Maximum</c> and <c>Increment</c> — as an invariant UTF-16 string.
/// </summary>
internal static class AvnDecimalValue
{
    public static string ToAbi(decimal value) =>
        value.ToString(CultureInfo.InvariantCulture);

    public static decimal FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
            throw new ArgumentNullException(nameof(value));
        return decimal.Parse(
            value,
            NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint,
            CultureInfo.InvariantCulture);
    }
}
