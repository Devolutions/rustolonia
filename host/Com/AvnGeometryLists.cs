using Avalonia;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Avalonia.Host.Com;

/// <summary>
/// Converts between the ABI's UTF-16 point list and an <c>IList&lt;Point&gt;</c>.
/// The wire format matches the geometry kinds: space-separated "x,y" pairs.
/// </summary>
public static class AvnPointList
{
    public static string? ToAbi(global::System.Collections.Generic.IList<Point>? value)
    {
        if (value is null)
            return null;
        return string.Join(" ", value.Select(p =>
            FormattableString.Invariant($"{p.X.ToString(CultureInfo.InvariantCulture)},{p.Y.ToString(CultureInfo.InvariantCulture)}")));
    }

    public static global::System.Collections.Generic.IList<Point>? FromAbi(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;
        var points = new List<Point>();
        foreach (var token in value.Split(' '))
        {
            if (string.IsNullOrWhiteSpace(token))
                continue;
            var parts = token.Split(',');
            if (parts.Length != 2)
                continue;
            if (double.TryParse(parts[0], NumberStyles.Float, CultureInfo.InvariantCulture, out var x) &&
                double.TryParse(parts[1], NumberStyles.Float, CultureInfo.InvariantCulture, out var y))
                points.Add(new Point(x, y));
        }
        return points;
    }
}

/// <summary>
/// Converts between the ABI's UTF-16 dash list and an
/// <c>AvaloniaList&lt;double&gt;?</c>: comma-separated doubles.
/// </summary>
public static class AvnDoubleList
{
    public static string? ToAbi(Avalonia.Collections.AvaloniaList<double>? value)
    {
        if (value is null)
            return null;
        return string.Join(",", value.Select(v => v.ToString(CultureInfo.InvariantCulture)));
    }

    public static Avalonia.Collections.AvaloniaList<double>? FromAbi(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;
        var result = new Avalonia.Collections.AvaloniaList<double>();
        foreach (var token in value.Split(','))
        {
            if (double.TryParse(token, NumberStyles.Float, CultureInfo.InvariantCulture, out var d))
                result.Add(d);
        }
        return result;
    }
}

/// <summary>
/// Converts between the ABI's UTF-16 feature string and a
/// <c>FontFeatureCollection</c> (whose Parse exists but whose ToString is
/// inherited, so the converter owns both halves).
/// </summary>
public static class AvnFontFeatures
{
    public static string? ToAbi(Avalonia.Media.FontFeatureCollection? value)
    {
        if (value is null)
            return null;
        return string.Join(",", value.Select(f => f.ToString()));
    }

    public static Avalonia.Media.FontFeatureCollection? FromAbi(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;
        try
        {
            return Avalonia.Media.FontFeatureCollection.Parse(value);
        }
        catch
        {
            return null;
        }
    }
}
