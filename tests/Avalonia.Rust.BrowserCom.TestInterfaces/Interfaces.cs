using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace BrowserCom.Testing;

public static class InterfaceIds
{
    public static readonly Guid Calculator = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0101");
    public static readonly Guid Calculator2 = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0102");
    public static readonly Guid Calculator3 = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0103");
    public static readonly Guid Calculator4 = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0108");
    public static readonly Guid Base = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0104");
    public static readonly Guid Derived = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0105");
    public static readonly Guid LocalDerived = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0106");
    public static readonly Guid Nested = new("52863C1B-A5D6-405A-B7C5-20F1CBAD0107");
}

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0101")]
public partial interface ICalculator
{
    [PreserveSig]
    int Increment(int value, out int result);

    [PreserveSig]
    int Echo(string? value, out string? result);

    [PreserveSig]
    int RoundTrip(ICalculator? value, out ICalculator? result);

    [PreserveSig]
    int InterfaceImplementation(int InterfaceInformation, out int InterfaceImplementationVtable);
}

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0103")]
public partial interface ICalculator3
{
    [PreserveSig]
    int Triple(int value, out int result);
}

[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0104")]
public partial interface IBase
{
    [PreserveSig]
    int BaseValue(int value, out int result);
}

public partial class ContractContainer
{
    [GeneratedComInterface]
    [Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0107")]
    public partial interface INested
    {
        [PreserveSig]
        int NestedValue(int value, out int result);
    }
}
