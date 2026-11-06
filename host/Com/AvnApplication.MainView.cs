using System;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Rust.Interop;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
    public int SetMainView(IAvnControl? view)
    {
        try
        {
#if AVN_HOST_ANDROID
            // The activity's root is the window presenter; it shows the main
            // view whenever no window is (see Android/AndroidWindowPresenter).
            Dispatcher.UIThread.VerifyAccess();
            var androidView = view is null
                ? null
                : (Control?)ProjectionRuntime.Unwrap(view) ?? throw new ObjectDisposedException(nameof(view));
            if (androidView is Window)
                return HResults.E_INVALIDARG;
            Android.AndroidWindowPresenter.SetMainView(androidView);
            return HResults.S_OK;
#else
            if (Application.Current?.ApplicationLifetime is not ISingleViewApplicationLifetime lifetime)
                return HResults.E_NOTIMPL;
            Dispatcher.UIThread.VerifyAccess();
            var control = view is null
                ? null
                : (Control?)ProjectionRuntime.Unwrap(view) ?? throw new ObjectDisposedException(nameof(view));
            if (control is Window)
                return HResults.E_INVALIDARG;
            var previous = lifetime.MainView;
            try
            {
                lifetime.MainView = control;
            }
            catch (Exception error)
            {
                try
                {
                    lifetime.MainView = previous;
                }
                catch (Exception restoreError)
                {
                    throw new AggregateException("Failed to restore the previous main view.", error, restoreError);
                }
                throw;
            }
            return HResults.S_OK;
#endif
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}
