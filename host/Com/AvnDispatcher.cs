using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

[GeneratedComClass]
public sealed partial class AvnDispatcher : IAvnDispatcher
{
    public int CheckAccess(out int value)
    {
        value = Dispatcher.UIThread.CheckAccess() ? 1 : 0;
        return HResults.S_OK;
    }

    public int Post(IAvnAction? action)
    {
        if (action is null)
            return HResults.E_POINTER;

        try
        {
            Dispatcher.UIThread.Post(() =>
            {
                var hr = action.Invoke();
                if (hr < 0)
                    throw new COMException($"Posted Rust callback failed with HRESULT 0x{hr:X8}.", hr);
            });
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}
