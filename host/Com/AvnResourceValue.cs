using System;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Media;

namespace Avalonia.Host.Com;

internal enum AvnResourceKind
{
    Null,
    Boolean,
    Integer,
    Double,
    String,
    Color,
}

[GeneratedComClass]
public sealed partial class AvnResourceValue : IAvnResourceValue
{
    private readonly object? _value;
    private readonly AvnResourceKind _kind;

    internal AvnResourceValue(object? value)
    {
        (_kind, _value) = value switch
        {
            null => (AvnResourceKind.Null, null),
            bool typed => (AvnResourceKind.Boolean, (object)typed),
            byte or sbyte or short or ushort or int or uint or long =>
                (AvnResourceKind.Integer, (object)Convert.ToInt64(value)),
            float or double or decimal =>
                (AvnResourceKind.Double, (object)Convert.ToDouble(value)),
            string typed => (AvnResourceKind.String, (object)typed),
            Color typed => (AvnResourceKind.Color, (object)typed),
            ISolidColorBrush typed => (AvnResourceKind.Color, (object)typed.Color),
            _ => throw new NotSupportedException(
                $"Resource value type '{value.GetType().FullName}' is not supported by the ABI."),
        };
    }

    public int GetKind(out int value)
    {
        value = (int)_kind;
        return HResults.S_OK;
    }

    public int GetBoolean(out int value) =>
        Get(AvnResourceKind.Boolean, out value, typed => (bool)typed ? 1 : 0);

    public int GetInteger(out long value) =>
        Get(AvnResourceKind.Integer, out value, typed => (long)typed);

    public int GetDouble(out double value) =>
        Get(AvnResourceKind.Double, out value, typed => (double)typed);

    public int GetString(out string? value) =>
        Get(AvnResourceKind.String, out value, typed => (string)typed);

    public int GetColor(out int argb) =>
        Get(AvnResourceKind.Color, out argb, typed => unchecked((int)((Color)typed).ToUInt32()));

    private int Get<T>(AvnResourceKind expected, out T value, Func<object, T> convert)
    {
        value = default!;
        if (_kind != expected || _value is null)
            return HResults.E_INVALIDARG;
        value = convert(_value);
        return HResults.S_OK;
    }
}
