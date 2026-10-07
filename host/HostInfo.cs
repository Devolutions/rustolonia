using System.Runtime.InteropServices;

namespace Avalonia.Host;

/// <summary>
/// Native layout written by <c>avn_get_host_info</c>. The caller sets
/// <see cref="StructSize"/> to the size it allocated so the struct can grow
/// later without breaking older callers. The string pointers refer to
/// NUL-terminated UTF-8 owned by the host for the life of the process; the
/// caller must not free them.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct HostInfoNative
{
    public uint StructSize;
    public uint Reserved;
    public byte* Version;
    public byte* AbiFingerprint;
}

internal static unsafe class HostInfo
{
    private static readonly byte* VersionUtf8 = (byte*)Marshal.StringToCoTaskMemUTF8(HostBuildInfo.Version);
    private static readonly byte* AbiFingerprintUtf8 = (byte*)Marshal.StringToCoTaskMemUTF8(HostBuildInfo.AbiFingerprint);

    internal static int Fill(HostInfoNative* info)
    {
        if (info is null)
            return HResults.E_POINTER;
        if (info->StructSize < (uint)sizeof(HostInfoNative))
            return HResults.E_INVALIDARG;

        info->StructSize = (uint)sizeof(HostInfoNative);
        info->Reserved = 0;
        info->Version = VersionUtf8;
        info->AbiFingerprint = AbiFingerprintUtf8;
        return HResults.S_OK;
    }
}
