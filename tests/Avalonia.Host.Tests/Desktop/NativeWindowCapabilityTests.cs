using System;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class NativeWindowCapabilityTests
{
    [Fact]
    public void Null_native_window_arguments_return_pointer_errors_and_zero_outputs()
    {
        var application = new AvnApplication();

        Assert.Equal(
            HResults.E_POINTER,
            application.GetNativeWindowHandle(null, out var handle, out var kind));
        Assert.Equal(0, handle);
        Assert.Equal(0, kind);
        Assert.Equal(HResults.E_POINTER, application.GetMacOSWindowNumber(null, out var number));
        Assert.Equal(0, number);
        Assert.Equal(HResults.E_POINTER, application.SetMacOSWindowIgnoresMouseEvents(null, 1));
    }

    [Fact]
    public void MacOS_run_teardown_disposes_the_native_windowing_platform()
    {
        var platform = new TrackingDisposable();

        AvnApplication.DisposeWindowingPlatform(isMacOS: true, platform: platform);

        Assert.True(platform.IsDisposed);
    }

    [Fact]
    public void Non_macOS_run_teardown_leaves_platform_disposal_unchanged()
    {
        var platform = new TrackingDisposable();

        AvnApplication.DisposeWindowingPlatform(isMacOS: false, platform: platform);

        Assert.False(platform.IsDisposed);
    }

    [Theory]
    [InlineData("HWND", 1)]
    [InlineData("XID", 2)]
    [InlineData("NSView", 3)]
    [InlineData("NSWindow", 4)]
    [InlineData("unknown", 0)]
    public void Native_handle_descriptors_have_stable_kind_values(string descriptor, int expected)
    {
        Assert.Equal(expected, AvnApplication.NativeHandleKind(descriptor));
    }

    [Fact]
    public void Show_window_without_activation_preserves_the_window_setting()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var window = new Window { ShowActivated = true };

        AvnApplication.ShowWindow(window, activate: false);

        Assert.True(window.IsVisible);
        Assert.False(window.IsActive);
        Assert.True(window.ShowActivated);
    }

    private sealed class TrackingDisposable : IDisposable
    {
        public bool IsDisposed { get; private set; }

        public void Dispose() => IsDisposed = true;
    }
}
