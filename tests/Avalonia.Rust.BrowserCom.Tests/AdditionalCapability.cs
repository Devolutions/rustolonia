using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace BrowserCom.Testing;

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0108")]
public partial interface ICalculator4
{
    [PreserveSig]
    int Quadruple(int value, out int result);
}

[GeneratedComClass]
internal sealed partial class CalculatorExtension : ICalculator, ICalculator2, ICalculator3, ICalculator4
{
    public int Increment(int value, out int result) { result = value + 1; return 0; }
    public int Echo(string? value, out string? result) { result = value; return 0; }
    public int RoundTrip(ICalculator? value, out ICalculator? result) { result = value; return 0; }
    public int InterfaceImplementation(int InterfaceInformation, out int InterfaceImplementationVtable)
    {
        InterfaceImplementationVtable = InterfaceInformation;
        return 0;
    }
    public int Double(int value, out int result) { result = value * 2; return 0; }
    public int Triple(int value, out int result) { result = value * 3; return 0; }
    public int Quadruple(int value, out int result) { result = value * 4; return 0; }
}
