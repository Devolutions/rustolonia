using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace BrowserCom.Testing;

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0102")]
public partial interface ICalculator2
{
    [PreserveSig]
    int Double(int value, out int result);
}

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0105")]
#pragma warning disable SYSLIB1230 // This fixture deliberately tests inheritance across assemblies.
public partial interface IDerived : IBase
#pragma warning restore SYSLIB1230
{
    [PreserveSig]
    int DerivedValue(int value, out int result);
}

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0106")]
public partial interface ILocalDerived : IDerived
{
    [PreserveSig]
    int LocalValue(int value, out int result);
}

[GeneratedComClass]
public sealed partial class Calculator : ICalculator, ICalculator2, ICalculator3, ILocalDerived, ContractContainer.INested
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
    public int BaseValue(int value, out int result) { result = value + 10; return 0; }
    public int DerivedValue(int value, out int result) { result = value + 20; return 0; }
    public int LocalValue(int value, out int result) { result = value + 30; return 0; }
    public int NestedValue(int value, out int result) { result = value + 40; return 0; }
}

public partial struct StructFixture
{
    public static object Create() => new Handler();

    [GeneratedComClass]
    private sealed partial class Handler : ICalculator3
    {
        public int Triple(int value, out int result) { result = value * 3; return 0; }
    }
}

[GeneratedComClass]
public partial class CalculatorBase : ICalculator3
{
    public int Triple(int value, out int result) { result = value * 3; return 0; }
}

[GeneratedComClass]
public sealed partial class CalculatorDerived : CalculatorBase, ICalculator2
{
    public int Double(int value, out int result) { result = value * 2; return 0; }
}

public static partial class NestedFixture
{
    public static object Create() => Container.Create();

    private static partial class Container
    {
        internal static object Create() => new Handler();

        [GeneratedComClass]
        private sealed partial class Handler : ICalculator3
        {
            public int Triple(int value, out int result) { result = value * 3; return 0; }
        }
    }
}
