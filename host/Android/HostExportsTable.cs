using System.Runtime.InteropServices;

namespace Avalonia.Host.Android;

/// <summary>
/// The bootstrap table handed to the application's Rust library. Mirrors
/// <c>rustolonia_sys::HostExports</c> field for field, like the iOS host's
/// table in <c>IosBootstrap.cs</c> (the Rust test
/// <c>in_process_table_layout_is_stable</c> pins the order): Android hosts are
/// not NativeAOT libraries that Rust opens by name, so the host passes the
/// same <see cref="Exports"/> entry points as function pointers instead.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct HostExportsTable
{
    public uint StructSize;
    public uint Reserved;
    public delegate* unmanaged<nint*, int> GetActivationFactory;
    public delegate* unmanaged<void*, void> Free;
    public delegate* unmanaged<int, char*> AllocUtf16;
    public delegate* unmanaged<nint*, int> GetLastError;
    public delegate* unmanaged<HostInfoNative*, int> GetHostInfo;

    internal static HostExportsTable Create() => new()
    {
        StructSize = (uint)sizeof(HostExportsTable),
        GetActivationFactory = &Exports.GetActivationFactory,
        Free = &Exports.Free,
        AllocUtf16 = &Exports.AllocUtf16,
        GetLastError = &Exports.GetLastError,
        GetHostInfo = &Exports.GetHostInfo,
    };
}
