using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class MacOSBackgroundTests
{
    [Fact]
    public void Background_choice_crosses_COM_and_is_staged_before_initialization()
    {
        var host = new AvnApplication();
        var wrappers = new StrategyBasedComWrappers();
        var unknown = wrappers.GetOrCreateComInterfaceForObject(host, CreateComInterfaceFlags.None);
        try
        {
            var application = (IAvnApplication)wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None);
            if (!OperatingSystem.IsMacOS())
            {
                Assert.Equal(HResults.E_NOTIMPL, application.SetMacOSBackground(1));
                Assert.Equal(HResults.E_NOTIMPL, application.PromoteMacOSToForeground());
                return;
            }
            Assert.False(host.MacOSBackground);
            Assert.Equal(HResults.S_OK, application.SetMacOSBackground(1));
            Assert.True(host.MacOSBackground);
            Assert.Equal(HResults.E_INVALIDARG, application.SetMacOSBackground(2));
            Assert.True(host.MacOSBackground);
            Assert.Equal(HResults.S_OK, application.SetMacOSBackground(0));
            Assert.False(host.MacOSBackground);
            host.CreateApplication();
            Assert.True(application.SetMacOSBackground(1) < 0);
            Assert.Contains("before Avalonia initialization", AbiError.Take());
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    [Fact]
    public void Initialized_application_rejects_staging_and_foreground_requires_initialization()
    {
        if (!OperatingSystem.IsMacOS())
            return;
        Assert.True(new AvnApplication().PromoteMacOSToForeground() < 0);
        Assert.Contains("must be initialized", AbiError.Take());
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        Assert.True(new AvnApplication().SetMacOSBackground(1) < 0);
        Assert.Contains("before Avalonia initialization", AbiError.Take());
    }
}
