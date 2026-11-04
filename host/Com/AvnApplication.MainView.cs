using System;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication : IAvnApplication13
{
    public int SetMainView(IAvnControl? view)
    {
        try
        {
            if (Application.Current?.ApplicationLifetime is not ISingleViewApplicationLifetime lifetime)
                return HResults.E_NOTIMPL;
            Dispatcher.UIThread.VerifyAccess();
            lifetime.MainView = view is null
                ? null
                : (Control?)ProjectionRuntime.Unwrap(view) ?? throw new ObjectDisposedException(nameof(view));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}
