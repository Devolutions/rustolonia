using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// ABI fixture that echoes every projected geometry value type back through nano-COM.
/// Like <see cref="IAvnEcho"/> this is not part of the projected object model, so adding
/// members here never widens a published control vtable.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D30")]
public partial interface IAvnGeometryEcho
{
    [PreserveSig]
    int EchoThickness(AvnThickness value, out AvnThickness result);

    [PreserveSig]
    int EchoCornerRadius(AvnCornerRadius value, out AvnCornerRadius result);

    [PreserveSig]
    int EchoSize(AvnSize value, out AvnSize result);

    [PreserveSig]
    int EchoPoint(AvnPoint value, out AvnPoint result);

    [PreserveSig]
    int EchoRect(AvnRect value, out AvnRect result);

    [PreserveSig]
    int EchoColor(AvnColor value, out AvnColor result);
}
