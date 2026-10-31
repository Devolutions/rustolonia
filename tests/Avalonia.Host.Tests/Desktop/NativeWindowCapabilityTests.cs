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
    public void Application8_is_a_separate_capability_with_a_stable_abi_shape()
    {
        Assert.Equal(
            Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA0"),
            typeof(IAvnApplication8).GUID);
        Assert.Equal(
            new[]
            {
                nameof(IAvnApplication8.GetNativeWindowHandle),
                nameof(IAvnApplication8.GetMacOSWindowNumber),
                nameof(IAvnApplication8.SetMacOSWindowIgnoresMouseEvents),
                nameof(IAvnApplication8.ShowWindow),
                nameof(IAvnApplication8.ActivateWindow),
            },
            typeof(IAvnApplication8).GetMethods().Select(method => method.Name));
    }

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
}
