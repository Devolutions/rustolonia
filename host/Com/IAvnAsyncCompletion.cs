using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnAsyncCompletion)]
public partial interface IAvnAsyncCompletion
{
    [PreserveSig]
    int Complete(
        long operationId,
        int hresult,
        int valueKind,
        long integerValue,
        double doubleValue,
        string? stringValue,
        string? error);
}
