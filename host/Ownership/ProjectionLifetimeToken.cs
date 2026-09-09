using Avalonia.Host.Ownership.MicroCom;
using MicroCom.Runtime;

namespace Avalonia.Host.Ownership;

internal sealed class ProjectionLifetimeToken(ProjectionObjectState state) :
    HostNativeOwned,
    IAvnMicroComLifetimeToken
{
    public nint GetNativePointer() =>
        MicroComRuntime.GetNativeIntPtr<IAvnMicroComLifetimeToken>(
            this,
            true);

    protected override void Destroyed() =>
        state.ReleaseNativeOwnership();
}
