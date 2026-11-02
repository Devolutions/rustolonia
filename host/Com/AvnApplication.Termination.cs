using System;
using System.Runtime.InteropServices;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Threading;
#if AVN_HOST_OSX
using Avalonia.Native;
#endif

namespace Avalonia.Host.Com;

public partial class AvnApplication : IAvnApplication11
{
    private static AvnApplication? s_terminationOwner;
    private IAvnTerminationHandler? _terminationHandler;
    private ClassicDesktopStyleApplicationLifetime? _terminationLifetime;
    private long _nextTerminationToken;
    private long _terminationToken;
    private int _terminationFlags;
    private bool _completingTermination;
    private bool _notifyingTermination;

    public int SetTerminationHandler(IAvnTerminationHandler? handler)
    {
        if (!OperatingSystem.IsMacOS())
            return HResults.E_NOTIMPL;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (_terminationToken != 0 || _completingTermination)
                throw new InvalidOperationException("Complete or cancel the pending termination before reconfiguration.");
            if (s_terminationOwner is { } owner && owner != this)
                throw new InvalidOperationException("Another application interface owns termination coordination.");
            if (handler is null)
            {
                ReleaseTerminationHandler();
                return HResults.S_OK;
            }
            var lifetime = Application.Current?.ApplicationLifetime as ClassicDesktopStyleApplicationLifetime
                ?? throw new InvalidOperationException("Install termination coordination inside the running application.");
            if (TerminationObjC.SupportsDeferredTermination() != 1)
                throw new NotSupportedException("The native companion does not support deferred termination.");
            ReleaseTerminationHandler();
            _terminationHandler = handler;
            _terminationLifetime = lifetime;
            lifetime.ShutdownRequested += TerminationRequested;
            lifetime.Exit += TerminationExit;
            s_terminationOwner = this;
#if AVN_HOST_OSX
            MacOSTermination.DeferRequested = NativeTerminationRequested;
#endif
            return HResults.S_OK;
        }
        catch (Exception error)
        {
            return AbiError.Capture(error);
        }
    }

    private void TerminationRequested(object? sender, ShutdownRequestedEventArgs args)
    {
        if (_completingTermination)
            return;
        args.Cancel = true;
        RequestTermination(args.IsOSShutdown ? 2 : 0);
    }

    private bool NativeTerminationRequested(bool isOSShutdown)
    {
        if (_completingTermination)
            return false;
        RequestTermination(1 | (isOSShutdown ? 2 : 0));
        return true;
    }

    private void RequestTermination(int flags)
    {
        Dispatcher.UIThread.VerifyAccess();
        if (_terminationToken == 0)
            _terminationToken = checked(++_nextTerminationToken);
        _terminationFlags |= flags;
        if (_notifyingTermination)
            return;
        _notifyingTermination = true;
        try
        {
            var hr = _terminationHandler!.OnRequested(_terminationToken, _terminationFlags);
            if (hr < 0)
                _ = AbiError.Capture(Marshal.GetExceptionForHR(hr)!);
        }
        finally
        {
            _notifyingTermination = false;
        }
        // Failure remains held: never accept termination when cleanup failed.
    }

    public int CompleteTermination(long token, int accept)
    {
        if (!OperatingSystem.IsMacOS())
            return HResults.E_NOTIMPL;
        if (accept is not (0 or 1))
            return HResults.E_INVALIDARG;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (token == 0 || token != _terminationToken || _completingTermination || _notifyingTermination)
                return HResults.E_INVALIDARG;
            var native = (_terminationFlags & 1) != 0;
            _terminationToken = 0;
            _terminationFlags = 0;
            if (accept == 0)
            {
                if (native)
                    ReplyToNativeTermination(false);
                return HResults.S_OK;
            }
            _completingTermination = true;
            try
            {
                // Cleanup was acknowledged; normal synchronous window/popup teardown must
                // precede AppKit's accepting reply and its WillTerminate notification.
                _terminationLifetime!.Shutdown();
                ReleaseTerminationHandler();
                if (native)
                    ReplyToNativeTermination(true);
            }
            catch
            {
                ReleaseTerminationHandler();
                if (native)
                    ReplyToNativeTermination(false);
                throw;
            }
            finally
            {
                _completingTermination = false;
            }
            return HResults.S_OK;
        }
        catch (Exception error)
        {
            return AbiError.Capture(error);
        }
    }

    private void TerminationExit(object? sender, ControlledApplicationLifetimeExitEventArgs args)
    {
        // Keep the lifetime available until CompleteTermination has finished Shutdown.
        if (!_completingTermination)
        {
            var native = (_terminationFlags & 1) != 0;
            ReleaseTerminationHandler();
            if (native)
                ReplyToNativeTermination(false);
        }
    }

    private void ReleaseTerminationHandler()
    {
        if (_terminationLifetime is { } lifetime)
        {
            lifetime.ShutdownRequested -= TerminationRequested;
            lifetime.Exit -= TerminationExit;
        }
        _terminationLifetime = null;
        _terminationHandler = null;
        _terminationToken = 0;
        _terminationFlags = 0;
        if (s_terminationOwner == this)
        {
            s_terminationOwner = null;
#if AVN_HOST_OSX
            MacOSTermination.DeferRequested = null;
#endif
        }
    }

    private static void ReplyToNativeTermination(bool accept)
    {
        var app = TerminationObjC.Send(TerminationObjC.GetClass("NSApplication"),
            TerminationObjC.Selector("sharedApplication"));
        TerminationObjC.Reply(app, TerminationObjC.Selector("replyToApplicationShouldTerminate:"), accept);
    }

    private static class TerminationObjC
    {
        [DllImport("libAvaloniaNative.dylib", EntryPoint = "avn_supports_deferred_termination")]
        internal static extern int SupportsDeferredTermination();
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_getClass")]
        internal static extern nint GetClass(string name);
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "sel_registerName")]
        internal static extern nint Selector(string name);
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
        internal static extern nint Send(nint receiver, nint selector);
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
        internal static extern void Reply(nint receiver, nint selector, [MarshalAs(UnmanagedType.I1)] bool accept);
    }
}
