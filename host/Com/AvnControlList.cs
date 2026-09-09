using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;

namespace Avalonia.Host.Com;

/// <summary>
/// Host-implemented <see cref="IAvnControlList"/>: a live, write-through view over
/// an <c>IList&lt;Control&gt;</c>-shaped collection such as CommandBar's command
/// lists. The CLR lists are <c>IList&lt;ICommandBarElement&gt;</c>, whose elements
/// are controls; the adapter works through the non-generic <see cref="IList"/> the
/// observable collections also implement. Reads go straight to the source
/// collection, and mutations persist into it because the host hands the same
/// wrapped instance back.
/// </summary>
[GeneratedComClass]
public sealed unsafe partial class AvnControlList : IAvnControlList
{
    private readonly IList? _value;

    private AvnControlList(IList? value) => _value = value;

    public static IAvnControlList? FromManaged(object? value) =>
        value is null ? null : new AvnControlList(value as IList);

    public static object? ToManaged(IAvnControlList? value) =>
        value is AvnControlList local ? local._value : null;

    private static Control? Unwrap(object? element) =>
        element as Control;

    public int GetCount(out int value)
    {
        value = 0;
        try
        {
            value = _value?.Count ?? 0;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int GetAt(int index, out IAvnControl? value)
    {
        value = null;
        try
        {
            if (_value is null)
                return HResults.E_POINTER;
            value = (IAvnControl?)ProjectionRuntime.Wrap(Unwrap(_value[index]));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int Add(IAvnControl? value)
    {
        try
        {
            if (_value is null)
                return HResults.E_POINTER;
            _value.Add((Control)ProjectionRuntime.Unwrap(value)!);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int IndexOf(IAvnControl? value, out int index)
    {
        index = -1;
        try
        {
            if (_value is null)
                return HResults.E_POINTER;
            index = _value.IndexOf((Control)ProjectionRuntime.Unwrap(value)!);
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
            if (_value is null)
                return HResults.E_POINTER;
            _value.RemoveAt(index);
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
            if (_value is null)
                return HResults.E_POINTER;
            _value.Clear();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }
}
