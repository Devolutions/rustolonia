#if AVN_HOST_LINUX_DRM
using System;
using System.Runtime.InteropServices;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.LinuxFramebuffer;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
    private void RunLinuxDrm(IAvnAppHandler handler)
    {
        using var terminate = PosixSignalRegistration.Create(PosixSignal.SIGTERM, OnSignal);
        using var interrupt = PosixSignalRegistration.Create(PosixSignal.SIGINT, OnSignal);
        var options = LinuxDrmSettings.FromEnvironment(Environment.GetEnvironmentVariable);
        RustHostPlatform.Configure(AppBuilder.Configure<HostApplication>())
            .AfterSetup(builder =>
            {
                var lifetime = builder.Instance?.ApplicationLifetime as IControlledApplicationLifetime
                    ?? throw new InvalidOperationException("The DRM host requires a controlled lifetime.");
                lifetime.Startup += (_, _) =>
                {
                    var started = handler.OnStarted();
                    if (started < 0)
                        Marshal.ThrowExceptionForHR(started);
                };
            })
            .StartLinuxDrm(SnapshotStartupArguments(), card: options.Card, options: new DrmOutputOptions
            {
                Scaling = options.Scaling,
                Orientation = options.Orientation,
            });
    }

    private static void OnSignal(PosixSignalContext context)
    {
        context.Cancel = true;
        Dispatcher.UIThread.Post(() => ShutdownLifetime(Application.Current?.ApplicationLifetime));
    }
}
#endif
