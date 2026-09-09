using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Com;

namespace Avalonia.Host.Com;

/// <summary>
/// Bridges the async populator CCW: the managed delegate runs to completion and
/// reports its items back through the CCW's completion interface, and the
/// completion's variant list materializes into the enumerable the control
/// consumes.
/// </summary>
internal static class AvnPopulatorBridge
{
    public static async Task RunAsync(
        Func<string?, CancellationToken, Task<IEnumerable<object>>> populator,
        long requestId,
        IAvnAsyncPopulatorCompletion completion,
        string? searchText)
    {
        try
        {
            var items = await populator(searchText, CancellationToken.None);
            var list = AvnObjectList.FromManaged(items);
            _ = completion.Complete(requestId, 0, list);
        }
        catch (Exception e)
        {
            _ = completion.Complete(requestId, Marshal.GetHRForException(e), null);
        }
    }

    public static IEnumerable<object> Materialize(IAvnVariantList items)
    {
        if (items.GetCount(out var count) < 0)
            return Array.Empty<object>();
        var result = new List<object>(count);
        for (var index = 0; index < count; index++)
        {
            if (items.GetAt(index, out var variant) < 0)
                break;
            result.Add(MaterializeVariant(ref variant)!);
        }
        return result;
    }

    internal static object? MaterializeVariant(ref AvnVariant variant)
    {
        try
        {
            return variant.ToObject();
        }
        finally
        {
            variant.FreeUtf16();
        }
    }
}
