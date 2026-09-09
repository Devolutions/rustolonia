using System;
using System.Runtime.InteropServices;

#if AVN_HOST_WIN32
using Avalonia.Win32;
#elif AVN_HOST_X11
using Avalonia.X11;
#elif AVN_HOST_OSX
using Avalonia.Native;
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
        builder.UseX11();
#elif AVN_HOST_OSX
        builder.UseAvaloniaNative();
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
