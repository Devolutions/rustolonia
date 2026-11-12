using System;
using System.Runtime.InteropServices;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
    internal bool MacOSBackground { get; private set; }

#if AVN_HOST_OSX
    internal MacOSPlatformOptions CreateMacOSPlatformOptions() => new()
    {
        ShowInDock = !MacOSBackground,
    };
#endif

    public int SetMacOSBackground(int enabled)
    {
        if (enabled is not (0 or 1))
            return HResults.E_INVALIDARG;
#if AVN_HOST_OSX
        try
        {
            if (_applicationCreated || Application.Current is not null)
                throw new InvalidOperationException("macOS background startup must be set before Avalonia initialization.");
            MacOSBackground = enabled != 0;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
#else
        return HResults.E_NOTIMPL;
#endif
    }

    public int PromoteMacOSToForeground()
    {
#if AVN_HOST_OSX
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (Application.Current is null)
                throw new InvalidOperationException("Avalonia must be initialized before macOS foreground promotion.");
            MacOS.ObjC.SetRegularApplicationPolicy();
            if (MacOS.ObjC.IsApplicationRunning())
                MacOS.ObjC.ActivateApplication();
            else
                Dispatcher.UIThread.Post(MacOS.ObjC.ActivateApplication);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
#else
        return HResults.E_NOTIMPL;
#endif
    }

#if AVN_HOST_OSX
    private static partial class MacOS
    {
        public static partial class ObjC
        {
            private static nint ApplicationHandle() =>
                SendPointer(GetClass("NSApplication"), Selector("sharedApplication"));

            public static void SetRegularApplicationPolicy()
            {
                var application = ApplicationHandle();
                if (SendInt(application, Selector("activationPolicy")) != 0 &&
                    SendBoolInt(application, Selector("setActivationPolicy:"), 0) == 0)
                    throw new InvalidOperationException("macOS rejected the Regular application activation policy.");
            }

            public static bool IsApplicationRunning() =>
                SendBool(ApplicationHandle(), Selector("isRunning")) != 0;

            public static void ActivateApplication() =>
                SendVoidBool(ApplicationHandle(), Selector("activateIgnoringOtherApps:"), 1);

            [LibraryImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_getClass", StringMarshalling = StringMarshalling.Utf8)]
            private static partial nint GetClass(string name);

            [LibraryImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
            private static partial nint SendPointer(nint receiver, nint selector);

            [LibraryImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
            private static partial byte SendBool(nint receiver, nint selector);

            [LibraryImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
            private static partial byte SendBoolInt(nint receiver, nint selector, nint value);
        }
    }
#endif
}
