using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Avalonia;
using Avalonia.Data.Converters;
using Avalonia.Media;
using Avalonia.Media.Imaging;

namespace NeoHtop.Presentation;

public sealed class UsageToBrushConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var usage = value switch
        {
            double d => d,
            float f => f,
            int i => i,
            _ => 0d,
        };
        if (usage > 90)
            return Brush("#F38BA8");
        if (usage > 75)
            return Brush("#FAB387");
        if (usage > 50)
            return Brush("#F9E2AF");
        return Brush("#89B4FA");
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();

    private static IBrush Brush(string hex) => SolidColorBrush.Parse(hex);
}

public sealed class SortGlyphConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var sort = value as string ?? "";
        var field = parameter as string ?? "";
        if (field.Length == 0 || !sort.StartsWith(field + " ", StringComparison.Ordinal))
            return "↕";
        return sort.Contains("Ascending", StringComparison.Ordinal) ? "↑" : "↓";
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class SortGlyphOpacityConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var sort = value as string ?? "";
        var field = parameter as string ?? "";
        return field.Length > 0 && sort.StartsWith(field + " ", StringComparison.Ordinal) ? 1d : 0.45d;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class RowHighlightConverter : IMultiValueConverter
{
    public object? Convert(IList<object?> values, Type targetType, object? parameter, CultureInfo culture)
    {
        var pinned = values.Count > 0 && values[0] is true;
        var high = values.Count > 1 && values[1] is true;
        if (pinned)
            return SolidColorBrush.Parse("#1989B4FA");
        if (high)
            return SolidColorBrush.Parse("#19F38BA8");
        return Brushes.Transparent;
    }
}

public sealed class BoolToAngleConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is true ? 45d : 0d;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class FreezeGlyphConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is true ? "\uE768" : "\uE769";

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class StatusToBrushConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var status = value as string ?? "";
        return status.Equals("Running", StringComparison.OrdinalIgnoreCase)
            ? SolidColorBrush.Parse("#A6E3A1")
            : SolidColorBrush.Parse("#CDD6F4");
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class StringEmptyToBoolConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        string.IsNullOrEmpty(value as string);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class Base64BitmapConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var text = value as string;
        if (string.IsNullOrEmpty(text))
            return null;
        try
        {
            var bytes = System.Convert.FromBase64String(text);
            return new Bitmap(new MemoryStream(bytes));
        }
        catch
        {
            return null;
        }
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public sealed class Int64TextConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value switch
        {
            long n => n.ToString(CultureInfo.InvariantCulture),
            int n => n.ToString(CultureInfo.InvariantCulture),
            _ => value?.ToString() ?? "",
        };

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var text = (value as string ?? "").Trim();
        if (text.Length == 0)
            return AvaloniaProperty.UnsetValue;
        if (long.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out var n))
            return n;
        return AvaloniaProperty.UnsetValue;
    }
}

public sealed class PercentLabelToDoubleConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var text = (value as string ?? "").Trim().TrimEnd('%');
        return double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var n)
            ? n
            : 0d;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}
