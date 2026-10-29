using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Avalonia.Data.Converters;
using Avalonia.Media;
using Avalonia.Media.Imaging;

namespace Jiffshot.Editor.Controls;

/// <summary>"#AARRGGBB" strings from Rust to brushes.</summary>
public sealed class ColorBrushConverter : IValueConverter
{
    public static readonly ColorBrushConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is string s && Color.TryParse(s, out var c) ? new SolidColorBrush(c) : Brushes.Transparent;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}

/// <summary>Thumbnail paths to bitmaps, cached by path and file time.</summary>
public sealed class ThumbnailConverter : IValueConverter
{
    public static readonly ThumbnailConverter Instance = new();
    private readonly Dictionary<string, (DateTime Time, Bitmap Image)> _cache = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not string path || path.Length == 0 || !File.Exists(path))
            return null;
        var time = File.GetLastWriteTimeUtc(path);
        if (_cache.TryGetValue(path, out var hit) && hit.Time == time)
            return hit.Image;
        try
        {
            using var stream = File.OpenRead(path);
            var image = Bitmap.DecodeToHeight(stream, 120);
            _cache[path] = (time, image);
            return image;
        }
        catch (Exception)
        {
            return null;
        }
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}

/// <summary>True when the bound string equals the parameter.</summary>
public sealed class EqualsConverter : IValueConverter
{
    public static readonly EqualsConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        string.Equals(value as string, parameter as string, StringComparison.Ordinal);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}

/// <summary>Seconds to m:ss.f.</summary>
public sealed class TimeConverter : IValueConverter
{
    public static readonly TimeConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var s = value is double d ? d : 0;
        return $"{(int)(s / 60)}:{s % 60:00.0}";
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}

/// <summary>A number times the parameter (a widths-from-heights helper).</summary>
public sealed class ScaleConverter : IValueConverter
{
    public static readonly ScaleConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        (value is double d ? d : 0) * (double.TryParse(parameter as string, NumberStyles.Float, CultureInfo.InvariantCulture, out var k) ? k : 1);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}

/// <summary>True for a non-empty string.</summary>
public sealed class NotEmptyConverter : IValueConverter
{
    public static readonly NotEmptyConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => value is string { Length: > 0 };

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => null;
}
