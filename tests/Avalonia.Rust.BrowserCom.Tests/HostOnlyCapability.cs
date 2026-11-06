using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace BrowserCom.Testing;

/// <summary>Listed in BrowserComHostOnlyFamilies: no native-object proxies are generated.</summary>
[GeneratedComInterface]
[Guid("52863C1B-A5D6-405A-B7C5-20F1CBAD0109")]
public partial interface IHostOnly
{
    [PreserveSig]
    int Value(out int result);
}

[GeneratedComClass]
internal sealed partial class HostOnlyService : IHostOnly
{
    public int Value(out int result) { result = 7; return 0; }
}
