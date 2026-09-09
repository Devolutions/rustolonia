using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Adapts a calendar's date collections to the generated
/// <see cref="IAvnDateTimeList"/> ABI: each element crosses as its int64
/// DateTime tick count. Reads enumerate the live collection; mutations write
/// through to it because the calendar owns the storage.
/// </summary>
[GeneratedComClass]
public sealed unsafe partial class AvnDateList : IAvnDateTimeList
{
    private readonly global::System.Collections.IList _value;

    private AvnDateList(global::System.Collections.IList value) => _value = value;

    public static IAvnDateTimeList? FromManaged(object? value) =>
        value is null ? null : new AvnDateList((global::System.Collections.IList)value);

    public static object? ToManaged(IAvnDateTimeList? value) =>
        value is AvnDateList local ? local._value : null;

    public int GetCount(out int value)
    {
        value = _value.Count;
        return HResults.S_OK;
    }

    public int GetAt(int index, out long value)
    {
        value = 0;
        try
        {
            if (index < 0 || index >= _value.Count)
                return unchecked((int)0x80070057);
            var date = _value[index];
            value = date is DateTime dateTime ? dateTime.Ticks : 0;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int Add(long value)
    {
        try
        {
            _value.Add(new DateTime(value, DateTimeKind.Utc));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }

    public int IndexOf(long value, out int index)
    {
        index = -1;
        try
        {
            var needle = new DateTime(value, DateTimeKind.Utc);
            for (var position = 0; position < _value.Count; position++)
            {
                if (_value[position] is DateTime date && date == needle)
                {
                    index = position;
                    break;
                }
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
            _value.Clear();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return Marshal.GetHRForException(e);
        }
    }
}
