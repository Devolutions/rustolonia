using System;
using System.Runtime.InteropServices;

#if AVN_HOST_WIN32
using Avalonia.Win32;
#elif AVN_HOST_X11
using Avalonia.X11;
#elif AVN_HOST_LINUX_DRM
#elif AVN_HOST_OSX
using Avalonia.Native;
#elif AVN_HOST_BROWSER
#elif AVN_HOST_IOS
using Avalonia.iOS;
#elif AVN_HOST_ANDROID
using Avalonia.Android;
#else
#error A supported Avalonia Rust host platform must be selected.
#endif

namespace Avalonia.Host;

internal static partial class RustHostPlatform
{
    internal static AppBuilder Configure(AppBuilder builder)
    {
#if AVN_HOST_WIN32
        builder.UseWin32();
#elif AVN_HOST_X11
#if AVN_HOST_STATIC_MUSL
        // Avoid GLX loading a second, dynamic Xlib beside the statically linked one.
        builder.UseX11().With(new X11PlatformOptions
        {
            RenderingMode = new[] { X11RenderingMode.Software }
        });
#else
        builder.UseX11();
#endif
#elif AVN_HOST_LINUX_DRM
        // StartLinuxDrm installs the direct-display windowing subsystem.
#elif AVN_HOST_OSX
        builder.UseAvaloniaNative();
#elif AVN_HOST_BROWSER
        // StartBrowserAppAsync installs the browser windowing subsystem after
        // loading Avalonia's JavaScript modules.
#elif AVN_HOST_IOS
        builder.UseiOS();
#elif AVN_HOST_ANDROID
        // Unused at run time: RustoloniaAndroidApplication sets Avalonia up
        // before the Rust code runs, so Run never builds an AppBuilder.
        builder.UseAndroid();
#endif
        return builder
            .UseSkia()
            .UseHarfBuzz();
    }

    internal static ThreadScope EnterThread() => new();

    internal readonly struct ThreadScope : IDisposable
    {
#if AVN_HOST_WIN32
        private readonly bool _uninitializeOle;

        public ThreadScope()
        {
            var result = OleInitialize(0);
            if (result < 0)
                Marshal.ThrowExceptionForHR(result);
            _uninitializeOle = true;
        }

        public void Dispose()
        {
            if (_uninitializeOle)
                OleUninitialize();
        }
#else
        public ThreadScope()
        {
        }

        public void Dispose()
        {
        }
#endif
    }

#if AVN_HOST_WIN32
    [LibraryImport("ole32.dll")]
    private static partial int OleInitialize(nint reserved);

    [LibraryImport("ole32.dll")]
    private static partial void OleUninitialize();
#endif
}
