using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication : IAvnApplication10
{
    private long _nextPopupToken;
    private readonly Dictionary<long, (Window Owner, Action Show)> _pendingPopups = new();

    public int SetNativePopupGate(IAvnWindow? window, IAvnNativePopupHandler? handler)
    {
        if (window is null)
            return HResults.E_POINTER;
        if (!OperatingSystem.IsMacOS())
            return HResults.E_NOTIMPL;
        return WithWindow(window, owner =>
        {
            if (owner.OpenedPopups.Count != 0)
                throw new InvalidOperationException("Close owner popups before changing their show gate.");
            if (owner.UseOverlayPopups && handler is not null)
                throw new InvalidOperationException("Disable embedded popup hosting before installing a native gate.");
            owner.NativePopupShowRequested = handler is null ? null : (popup, show) =>
            {
                var handle = popup.TryGetPlatformHandle();
                if (handle is null || handle.HandleDescriptor != "NSWindow" || handle.Handle == 0)
                    throw new InvalidOperationException("Native popup has no borrowed NSWindow handle.");
                var number = MacOS.ObjC.GetWindowNumber(handle.Handle);
                if (number <= 0)
                    throw new InvalidOperationException("Native popup has no window number.");
                var token = checked(++_nextPopupToken);
                _pendingPopups.Add(token, (owner, show));
                void Closed(object? sender, EventArgs args)
                {
                    popup.Closed -= Closed;
                    _pendingPopups.Remove(token);
                    var hr = handler.OnClosed(token);
                    if (hr < 0)
                        _ = AbiError.Capture(Marshal.GetExceptionForHR(hr)!);
                }
                popup.Closed += Closed;
                var result = handler.OnPreparing(token, number);
                if (result < 0)
                {
                    _pendingPopups.Remove(token);
                    Marshal.ThrowExceptionForHR(result);
                }
            };
        });
    }

    public int AcknowledgeNativePopup(long token)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (!_pendingPopups.Remove(token, out var pending))
                return HResults.E_INVALIDARG;
            pending.Show();
            return HResults.S_OK;
        }
        catch (Exception error)
        {
            return AbiError.Capture(error);
        }
    }
}
