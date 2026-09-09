using System;
using System.Globalization;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects a nullable <see cref="DateTime"/> calendar day — <c>Calendar.SelectedDate</c> —
/// as an ISO-8601 <c>yyyy-MM-dd</c> string. Unlike <see cref="AvnDateTimeOffset"/> this is a
/// civil date, not an instant: there is no time-of-day and no offset on the wire.
/// </summary>
internal static class AvnCalendarDate
{
    public static string? ToAbi(DateTime? value) =>
        value is { } date ? AvnCalendarDateValue.ToAbi(date) : null;

    public static DateTime? FromAbi(string? value) =>
        string.IsNullOrEmpty(value) ? null : AvnCalendarDateValue.FromAbi(value);
}

/// <summary>
/// Projects a non-nullable <see cref="DateTime"/> calendar day — <c>Calendar.DisplayDate</c> —
/// as an ISO-8601 <c>yyyy-MM-dd</c> string.
/// </summary>
internal static class AvnCalendarDateValue
{
    public static string ToAbi(DateTime value) =>
        value.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

    public static DateTime FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
        {
            throw new ArgumentNullException(
                nameof(value),
                "This date has no absent state, so it cannot be cleared. Pass yyyy-MM-dd.");
        }

        if (!DateTime.TryParseExact(
                value,
                "yyyy-MM-dd",
                CultureInfo.InvariantCulture,
                DateTimeStyles.None,
                out var parsed))
        {
            throw new FormatException(
                $"Date '{value}' is not an ISO-8601 calendar day. Use yyyy-MM-dd.");
        }

        return DateTime.SpecifyKind(parsed, DateTimeKind.Unspecified);
    }
}
