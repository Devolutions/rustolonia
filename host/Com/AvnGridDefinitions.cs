using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;

namespace Avalonia.Host.Com;

// A track may append @SharedSizeGroup to its length. Ordinary Avalonia length
// lists retain their existing syntax and parsing behavior.
internal static class AvnGridDefinitions
{
    public static IEnumerable<(GridLength Length, string? Group)> Parse(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (!value.Contains('@'))
        {
            return GridLength.ParseLengths(value).Select(length => (length, (string?)null));
        }

        return value.Split(',').Select(track =>
        {
            var parts = track.Split('@');
            if (parts.Length > 2 || parts[0].Trim().Length == 0 ||
                (parts.Length == 2 && parts[1].Trim().Length == 0))
            {
                throw new FormatException("Invalid grid track definition.");
            }

            return (GridLength.Parse(parts[0].Trim()), parts.Length == 2 ? parts[1].Trim() : null);
        });
    }

    public static string Format(GridLength length, string? group) =>
        group is null ? length.ToString() : $"{length}@{group}";
}

internal static class AvnColumnDefinitions
{
    public static string ToAbi(ColumnDefinitions value) =>
        string.Join(",", value.Select(column => AvnGridDefinitions.Format(column.Width, column.SharedSizeGroup)));

    public static ColumnDefinitions FromAbi(string value)
    {
        var result = new ColumnDefinitions();
        foreach (var (length, group) in AvnGridDefinitions.Parse(value))
        {
            result.Add(new ColumnDefinition(length) { SharedSizeGroup = group });
        }

        return result;
    }
}

internal static class AvnRowDefinitions
{
    public static string ToAbi(RowDefinitions value) =>
        string.Join(",", value.Select(row => AvnGridDefinitions.Format(row.Height, row.SharedSizeGroup)));

    public static RowDefinitions FromAbi(string value)
    {
        var result = new RowDefinitions();
        foreach (var (length, group) in AvnGridDefinitions.Parse(value))
        {
            result.Add(new RowDefinition(length) { SharedSizeGroup = group });
        }

        return result;
    }
}
