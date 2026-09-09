using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComInterface]
[Guid(AvnGuids.IAvnAction)]
public partial interface IAvnAction
{
    [PreserveSig]
    int Invoke();
}

[GeneratedComInterface]
[Guid(AvnGuids.IAvnDispatcher)]
public partial interface IAvnDispatcher
{
    [PreserveSig]
    int CheckAccess(out int value);

    [PreserveSig]
    int Post(IAvnAction? action);
}
