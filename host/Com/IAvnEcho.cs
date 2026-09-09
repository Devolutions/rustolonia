using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnEcho)]
public partial interface IAvnEcho
{
    [PreserveSig]
    int Ping(int value, out int result);

    [PreserveSig]
    int EchoString(string? input, out string? output);

    [PreserveSig]
    int Fail();
}
