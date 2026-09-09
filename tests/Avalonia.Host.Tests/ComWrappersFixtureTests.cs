using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Host.Com;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class ComWrappersFixtureTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Ping_increments_value_through_ccw_rcw()
    {
        var echo = new AvnEcho();
        var unk = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unk);

        try
        {
            var rcw = (IAvnEcho)s_wrappers.GetOrCreateObjectForComInstance(unk, CreateObjectFlags.None);
            var hr = rcw.Ping(41, out var result);
            Assert.Equal(0, hr);
            Assert.Equal(42, result);
        }
        finally
        {
            Marshal.Release(unk);
        }
    }

    [Fact]
    public void EchoString_roundtrips_utf16()
    {
        var echo = new AvnEcho();
        var unk = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        try
        {
            var rcw = (IAvnEcho)s_wrappers.GetOrCreateObjectForComInstance(unk, CreateObjectFlags.None);
            var hr = rcw.EchoString("hello nano-COM", out var output);
            Assert.Equal(0, hr);
            Assert.Equal("hello nano-COM", output);
        }
        finally
        {
            Marshal.Release(unk);
        }
    }

    [Fact]
    public void Fail_returns_fixture_hresult()
    {
        var echo = new AvnEcho();
        var unk = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        try
        {
            var rcw = (IAvnEcho)s_wrappers.GetOrCreateObjectForComInstance(unk, CreateObjectFlags.None);
            Assert.Equal(unchecked((int)0xA7A70001), rcw.Fail());
        }
        finally
        {
            Marshal.Release(unk);
        }
    }

    [Fact]
    public void QueryInterface_identity_is_stable()
    {
        var echo = new AvnEcho();
        var unk1 = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        var unk2 = s_wrappers.GetOrCreateComInterfaceForObject(echo, CreateComInterfaceFlags.None);
        try
        {
            Assert.Equal(unk1, unk2);
        }
        finally
        {
            Marshal.Release(unk1);
            Marshal.Release(unk2);
        }
    }

    [Fact]
    public void Final_native_release_has_no_deterministic_managed_notification()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        var button = Assert.IsType<AvnButton>(projected);
        var handler = new BaselineClickHandler();
        Assert.Equal(0, button.AdviseClick(handler, out var subscriptionId));

        var pointer = s_wrappers.GetOrCreateComInterfaceForObject(
            button,
            CreateComInterfaceFlags.None);
        var beforeRelease = ProjectionDiagnostics.Capture();

        Assert.Equal(0, Marshal.Release(pointer));

        var afterRelease = ProjectionDiagnostics.Capture();
        Assert.Equal(
            beforeRelease.NativeOwnershipReleases,
            afterRelease.NativeOwnershipReleases);
        Assert.Equal(
            beforeRelease.ActiveSubscriptions,
            afterRelease.ActiveSubscriptions);

        Assert.Equal(0, button.UnadviseClick(subscriptionId));
        Assert.Equal(
            beforeRelease.ActiveSubscriptions - 1,
            ProjectionDiagnostics.Capture().ActiveSubscriptions);
    }

    [Fact]
    public void Factory_creates_echo()
    {
        var factory = new AvnActivationFactory();
        var hr = factory.CreateEcho(out var echo);
        Assert.Equal(0, hr);
        Assert.NotNull(echo);
        Assert.Equal(0, echo!.Ping(1, out var result));
        Assert.Equal(2, result);
    }

    [Fact]
    public void Factory_creates_application()
    {
        var factory = new AvnActivationFactory();
        var hr = factory.CreateApplication(out var app);
        Assert.Equal(0, hr);
        Assert.NotNull(app);
    }

    [Fact]
    public void HostApplication_loads_fluent()
    {
        var app = new HostApplication();
        app.Initialize();
        Assert.NotEmpty(app.Styles);
    }

    private sealed class BaselineClickHandler : IAvnButtonClickHandler
    {
        public int Invoke() => 0;
    }
}
