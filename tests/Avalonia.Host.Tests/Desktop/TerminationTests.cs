using System;
using System.Collections.Generic;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class TerminationTests
{
    private sealed class Handler(Action<long, int> callback) : IAvnTerminationHandler
    {
        public int OnRequested(long token, int flags)
        {
            callback(token, flags);
            return HResults.S_OK;
        }
    }

    [Fact]
    public void Termination_rejects_invalid_tokens()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = new AvnApplication();
        Assert.Single(typeof(IAvnTerminationHandler).GetMethods());
        Assert.Equal(OperatingSystem.IsMacOS() ? HResults.E_INVALIDARG : HResults.E_NOTIMPL,
            application.CompleteTermination(42, 1));
    }

    [Fact]
    public void Tray_only_requests_are_held_coalesced_cancelled_and_recreated()
    {
        if (!OperatingSystem.IsMacOS())
            return;
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var lifetime = new ClassicDesktopStyleApplicationLifetime();
        Avalonia.Application.Current!.ApplicationLifetime = lifetime;
        var application = new AvnApplication();
        var tokens = new List<long>();
        var handler = new Handler((token, flags) =>
        {
            Assert.Equal(0, flags);
            tokens.Add(token);
            Assert.Equal(HResults.E_INVALIDARG, application.CompleteTermination(token, 1));
        });
        Assert.Equal(HResults.S_OK, application.SetTerminationHandler(handler));
        try
        {
            Assert.True(new AvnApplication().SetTerminationHandler(handler) < 0);
            Assert.False(lifetime.TryShutdown());
            Assert.False(lifetime.TryShutdown());
            Assert.Equal(tokens[0], tokens[1]);
            Assert.True(application.SetTerminationHandler(null) < 0);
            Assert.Equal(HResults.S_OK, application.CompleteTermination(tokens[0], 0));
            Assert.Equal(HResults.E_INVALIDARG, application.CompleteTermination(tokens[0], 0));
            Assert.False(lifetime.TryShutdown());
            Assert.NotEqual(tokens[0], tokens[2]);
            Assert.Equal(HResults.S_OK, application.CompleteTermination(tokens[2], 0));
        }
        finally
        {
            Assert.Equal(HResults.S_OK, application.SetTerminationHandler(null));
        }
    }
}
