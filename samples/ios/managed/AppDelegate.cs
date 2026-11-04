using System;
using System.Runtime.InteropServices;
using Avalonia.Host;
using Avalonia.iOS;
#if RUSTOLONIA_IOS_SMOKE
using Avalonia;
using Avalonia.Threading;
#endif
using Foundation;
using UIKit;

namespace Rustolonia.IosSample;

[Register("AppDelegate")]
public sealed class AppDelegate : AvaloniaAppDelegate<HostApplication>
{
    [DllImport("__Internal", EntryPoint = "rustolonia_ios_stop")]
    private static extern int StopRust();

#if RUSTOLONIA_IOS_SMOKE
    protected override AppBuilder CustomizeAppBuilder(AppBuilder builder) =>
        base.CustomizeAppBuilder(builder).AfterSetup(_ =>
            Dispatcher.UIThread.Post(IosSmokeTest.Begin, DispatcherPriority.Background));
#endif

    [Export("applicationWillTerminate:")]
    public void WillTerminate(UIApplication application)
    {
        var result = StopRust();
        if (result < 0)
            Console.Error.WriteLine($"Rust iOS application shutdown failed (0x{result:X8}).");
    }
}
