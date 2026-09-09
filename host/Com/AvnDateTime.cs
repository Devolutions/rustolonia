using System;
using System.Globalization;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects a nullable <see cref="DateTimeOffset"/> — <c>DatePicker.SelectedDate</c> — as an
/// ISO-8601 string, because the projection has no date/time ABI shape of its own.
/// </summary>
/// <remarks>
/// <para>
/// The wire form is the invariant round-trip format <c>"o"</c>:
/// <c>yyyy-MM-ddTHH:mm:ss.fffffffK</c>, for example
/// <c>2026-09-03T00:00:00.0000000-04:00</c>. Reading always produces that exact shape, so a
/// value written and read back is byte-identical.
/// </para>
/// <para>
/// Writing is more forgiving than reading, but only within ISO-8601: a bare <c>yyyy-MM-dd</c>,
/// an added <c>THH:mm</c>, <c>THH:mm:ss</c> or <c>THH:mm:ss.fffffff</c>, and an optional
/// <c>Z</c> or <c>+hh:mm</c> offset. A locale spelling such as <c>03/09/2026</c> is rejected
/// with <see cref="FormatException"/> rather than resolved by whichever culture is installed. A
/// date with no offset is read as a local-time date rather than as UTC, matching what a
/// <c>DatePicker</c> means by "the selected day". A null or empty string clears the selection;
/// there is no "empty date" state.
/// </para>
/// </remarks>
internal static class AvnDateTimeOffset
{
    public static string? ToAbi(DateTimeOffset? value) =>
        value?.ToString("o", CultureInfo.InvariantCulture);

    public static DateTimeOffset? FromAbi(string? value) =>
        string.IsNullOrEmpty(value) ? null : AvnDateTimeOffsetValue.FromAbi(value);
}

/// <summary>
/// Projects a non-nullable <see cref="DateTimeOffset"/> — <c>DatePicker.MinYear</c> and
/// <c>DatePicker.MaxYear</c> — as an ISO-8601 string.
/// </summary>
/// <remarks>
/// Identical wire form and parse rules to <see cref="AvnDateTimeOffset"/>, minus the null: the
/// slot always carries a value, so a null or empty string fails the call with
/// <see cref="ArgumentNullException"/> instead of clearing anything.
/// </remarks>
internal static class AvnDateTimeOffsetValue
{
    /// <summary>
    /// The ISO-8601 spellings accepted on the way in. Nothing else is: a locale date such as
    /// <c>03/09/2026</c> is ambiguous between March 9th and the 3rd of September, so it is
    /// rejected rather than resolved by whichever culture happens to be installed.
    /// </summary>
    private static readonly string[] s_formats =
    [
        "yyyy-MM-dd",
        "yyyy-MM-ddK",
        "yyyy-MM-ddTHH:mm",
        "yyyy-MM-ddTHH:mmK",
        "yyyy-MM-ddTHH:mm:ss",
        "yyyy-MM-ddTHH:mm:ssK",
        "yyyy-MM-ddTHH:mm:ss.FFFFFFF",
        "yyyy-MM-ddTHH:mm:ss.FFFFFFFK",
    ];

    public static string ToAbi(DateTimeOffset value) =>
        value.ToString("o", CultureInfo.InvariantCulture);

    public static DateTimeOffset FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
        {
            throw new ArgumentNullException(
                nameof(value),
                "This date has no absent state, so it cannot be cleared. Pass an ISO-8601 date.");
        }

        if (!DateTimeOffset.TryParseExact(
                value,
                s_formats,
                CultureInfo.InvariantCulture,
                DateTimeStyles.AssumeLocal,
                out var parsed))
        {
            throw new FormatException(
                $"Date '{value}' is not ISO-8601. Use yyyy-MM-dd, optionally followed by " +
                "THH:mm[:ss[.fffffff]] and a Z or +hh:mm offset.");
        }

        return parsed;
    }
}

/// <summary>
/// Projects a nullable <see cref="TimeSpan"/> — <c>TimePicker.SelectedTime</c> — as an ISO-8601
/// wall-clock time string.
/// </summary>
/// <remarks>
/// <para>
/// The wire form is <c>HH:mm:ss</c> in 24-hour invariant form, for example <c>17:04:00</c>. A
/// <c>TimePicker</c>'s selection is a time of day rather than a duration, so this is
/// ISO-8601's extended time-of-day spelling and not an ISO-8601 <c>PnDTnHnMnS</c> duration.
/// Sub-second precision is not part of the wire form: a value carrying one is rejected rather
/// than silently truncated.
/// </para>
/// <para>
/// Writing accepts <c>HH:mm</c> as well as <c>HH:mm:ss</c>; reading always produces
/// <c>HH:mm:ss</c>. The value must be in <c>[00:00:00, 24:00:00)</c> — a negative or
/// day-spanning span is not a time of day and fails with
/// <see cref="ArgumentOutOfRangeException"/>. A null or empty string clears the selection.
/// </para>
/// </remarks>
internal static class AvnTimeSpan
{
    private static readonly string[] s_formats = ["hh\\:mm\\:ss", "hh\\:mm"];

    public static string? ToAbi(TimeSpan? value)
    {
        if (value is not { } time)
            return null;
        if (time < TimeSpan.Zero || time >= TimeSpan.FromDays(1))
        {
            throw new ArgumentOutOfRangeException(
                nameof(value),
                time,
                "A projected time of day must be within [00:00:00, 24:00:00).");
        }

        return time.ToString("hh\\:mm\\:ss", CultureInfo.InvariantCulture);
    }

    public static TimeSpan? FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
            return null;
        if (!TimeSpan.TryParseExact(
                value,
                s_formats,
                CultureInfo.InvariantCulture,
                TimeSpanStyles.None,
                out var time))
        {
            throw new FormatException(
                $"Time '{value}' is not a 24-hour ISO-8601 time of day. Use HH:mm:ss or HH:mm.");
        }

        return time;
    }
}
