using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Adapts an <see cref="IEnumerable"/> item source to the generated
/// <see cref="IAvnVariantList"/> ABI. Reads enumerate the source live; the
/// first mutation materializes the items into a shadow list that becomes the
/// adapter's own source from then on. Assign the adapter back (or any list)
/// through the setter to persist the mutation into the control.
/// </summary>
[GeneratedComClass]
public sealed unsafe partial class AvnObjectList : IAvnVariantList, IAvnSelectedVariantList
{
    private IEnumerable? _source;
    private List<object?>? _shadow;

    private AvnObjectList(IEnumerable? source) => _source = source;

    // The generated marshal signatures pass object? because several members share
    // this adapter with different CLR collection types.
    public static AvnObjectList? FromManaged(object? value) =>
        value is null ? null : new AvnObjectList(value as IEnumerable);

    public static object? ToManaged(IAvnVariantList? value) =>
        value is AvnObjectList local ? local.Materialized() : null;

    public static object? ToManaged(IAvnSelectedVariantList? value) =>
        value is AvnObjectList local ? local.Materialized() : null;

    private IEnumerable Materialized()
    {
        if (_shadow is null)
        {
            _shadow = _source is null ? [] : _source.Cast<object?>().ToList();
            _source = null;
        }
        return _shadow;
    }

    private List<object?> Materialize()
    {
        if (_shadow is null)
        {
            _shadow = _source is null ? [] : _source.Cast<object?>().ToList();
            _source = null;
        }
        return _shadow;
    }

    private IEnumerable Current =>
        _shadow is not null
            ? _shadow
            : _source ?? (IEnumerable)Array.Empty<object?>();

    public int GetCount(out int value)
    {
        value = 0;
        try
        {
            value = _shadow is { } shadow
                ? shadow.Count
                : _source?.Cast<object?>().Count() ?? 0;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int GetAt(int index, out AvnVariant value)
    {
        value = default;
        try
        {
            if (index < 0)
                return unchecked((int)0x80070057);
            if (_shadow is { } shadow)
            {
                if (index >= shadow.Count)
                    return unchecked((int)0x80070057);
                value = AvnVariant.FromObject(shadow[index]);
                return HResults.S_OK;
            }
            foreach (var (item, position) in _source!.Cast<object?>().Select((item, position) => (item, position)))
            {
                if (position == index)
                {
                    value = AvnVariant.FromObject(item);
                    return HResults.S_OK;
                }
            }
            return unchecked((int)0x80070057);
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int Add(AvnVariant value)
    {
        try
        {
            Materialize().Add(value.ToObject());
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int IndexOf(AvnVariant value, out int index)
    {
        index = -1;
        try
        {
            var needle = value.ToObject();
            var items = _shadow is { } shadow
                ? shadow
                : _source?.Cast<object?>() ?? Enumerable.Empty<object?>();
            var position = 0;
            foreach (var item in items)
            {
                if (Equals(item, needle))
                {
                    index = position;
                    return HResults.S_OK;
                }
                position++;
            }
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int RemoveAt(int index)
    {
        try
        {
            var shadow = Materialize();
            if (index < 0 || index >= shadow.Count)
                return unchecked((int)0x80070057);
            shadow.RemoveAt(index);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int Clear()
    {
        try
        {
            Materialize().Clear();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }
}
