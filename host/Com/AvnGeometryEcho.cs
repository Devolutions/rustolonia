using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// Round-trips the projected geometry value types through their managed Avalonia
/// equivalents so the blittable ABI structs are exercised end to end.
/// </summary>
[GeneratedComClass]
public partial class AvnGeometryEcho : IAvnGeometryEcho
{
    public int EchoThickness(AvnThickness value, out AvnThickness result)
    {
        result = AvnThickness.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }

    public int EchoCornerRadius(AvnCornerRadius value, out AvnCornerRadius result)
    {
        result = AvnCornerRadius.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }

    public int EchoSize(AvnSize value, out AvnSize result)
    {
        result = AvnSize.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }

    public int EchoPoint(AvnPoint value, out AvnPoint result)
    {
        result = AvnPoint.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }

    public int EchoRect(AvnRect value, out AvnRect result)
    {
        result = AvnRect.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }

    public int EchoColor(AvnColor value, out AvnColor result)
    {
        result = AvnColor.FromAvalonia(value.ToAvalonia());
        return HResults.S_OK;
    }
}
