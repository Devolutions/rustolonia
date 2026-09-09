using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComClass]
public partial class AvnEcho : IAvnEcho
{
    public int Ping(int value, out int result)
    {
        result = value + 1;
        return HResults.S_OK;
    }

    public int EchoString(string? input, out string? output)
    {
        if (input is null)
        {
            output = null;
            return HResults.E_INVALIDARG;
        }

        output = input;
        return HResults.S_OK;
    }

    public int Fail() => HResults.AVN_E_FIXTURE;
}
