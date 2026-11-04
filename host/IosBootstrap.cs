#if AVN_HOST_IOS
using System;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host;

internal static unsafe class IosBootstrap
{
    [StructLayout(LayoutKind.Sequential)]
    private struct HostExports
    {
        public uint StructSize;
        public uint Reserved;
        public nint GetActivationFactory;
        public nint Free;
        public nint AllocUtf16;
        public nint GetLastError;
        public nint GetHostInfo;
    }

    [DllImport("__Internal", EntryPoint = "rustolonia_ios_main")]
    private static extern int StartRust(ref HostExports exports);

    // Rust callbacks reach ComObject's explicit interface implementations only
    // through NativeAOT's dynamic dispatch, so they must survive trimming.
    [DynamicDependency(DynamicallyAccessedMemberTypes.NonPublicMethods, typeof(ComObject))]
    internal static void Start()
    {
        var exports = new HostExports
        {
            StructSize = (uint)sizeof(HostExports),
            GetActivationFactory = (nint)(delegate* unmanaged<nint*, int>)&Exports.GetActivationFactory,
            Free = (nint)(delegate* unmanaged<void*, void>)&Exports.Free,
            AllocUtf16 = (nint)(delegate* unmanaged<int, char*>)&Exports.AllocUtf16,
            GetLastError = (nint)(delegate* unmanaged<nint*, int>)&Exports.GetLastError,
            GetHostInfo = (nint)(delegate* unmanaged<HostInfoNative*, int>)&Exports.GetHostInfo,
        };
        var result = StartRust(ref exports);
        if (result < 0)
            throw new InvalidOperationException($"Rust iOS application startup failed (0x{result:X8}).");
    }
}
#endif
