using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnApplication12)]
public partial interface IAvnApplication12
{
    [PreserveSig]
    int SetFileDragHandler(IAvnControl? target, IAvnFileDragHandler? handler);
    /// <summary>JSON array of absolute file URLs, at least one; UI thread.</summary>
    [PreserveSig]
    int CompleteFileDrag(long token, string? fileUrlsJson);
    [PreserveSig]
    int CancelFileDrag(long token);
}

[GeneratedComInterface]
[Guid(AvnGuids.IAvnFileDragHandler)]
public partial interface IAvnFileDragHandler
{
    [PreserveSig]
    int OnPreparing(long token);
    /// <summary>effect: 0 cancelled/no drop, 1 copied; status HRESULT.</summary>
    [PreserveSig]
    int OnCompleted(long token, int effect, int status);
}
