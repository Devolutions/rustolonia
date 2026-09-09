using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnResourceValue)]
public partial interface IAvnResourceValue
{
    [PreserveSig]
    int GetKind(out int value);

    [PreserveSig]
    int GetBoolean(out int value);

    [PreserveSig]
    int GetInteger(out long value);

    [PreserveSig]
    int GetDouble(out double value);

    [PreserveSig]
    int GetString(out string? value);

    [PreserveSig]
    int GetColor(out int argb);
}
