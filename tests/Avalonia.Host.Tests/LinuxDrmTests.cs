using System;
using System.Collections.Generic;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Com;
using Avalonia.Platform;
using Xunit;

namespace Avalonia.Host.Tests;

public class LinuxDrmTests
{
    [Fact]
    public void Defaults_use_the_preferred_display_without_rotation()
    {
        var settings = LinuxDrmSettings.FromEnvironment(_ => null);
        Assert.Null(settings.Card);
        Assert.Equal(1.0, settings.Scaling);
        Assert.Equal(SurfaceOrientation.Rotation0, settings.Orientation);
    }

    [Theory]
    [InlineData("0", SurfaceOrientation.Rotation0)]
    [InlineData("90", SurfaceOrientation.Rotation90)]
    [InlineData("180", SurfaceOrientation.Rotation180)]
    [InlineData("270", SurfaceOrientation.Rotation270)]
    public void Explicit_display_settings_are_respected(string rotation, SurfaceOrientation expected)
    {
        var values = new Dictionary<string, string>
        {
            ["RUSTOLONIA_DRM_CARD"] = "/dev/dri/card1",
            ["RUSTOLONIA_DRM_SCALING"] = "1.5",
            ["RUSTOLONIA_DRM_ROTATION"] = rotation,
        };
        var settings = LinuxDrmSettings.FromEnvironment(name => values.GetValueOrDefault(name));
        Assert.Equal("/dev/dri/card1", settings.Card);
        Assert.Equal(1.5, settings.Scaling);
        Assert.Equal(expected, settings.Orientation);
    }

    [Theory]
    [InlineData("RUSTOLONIA_DRM_CARD", "")]
    [InlineData("RUSTOLONIA_DRM_CARD", "/dev/fb0")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "0")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "-1")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "NaN")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "Infinity")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "1,5")]
    [InlineData("RUSTOLONIA_DRM_SCALING", "")]
    [InlineData("RUSTOLONIA_DRM_ROTATION", "45")]
    public void Invalid_settings_fail_instead_of_falling_back(string variable, string value)
    {
        var error = Assert.Throws<ArgumentException>(() =>
            LinuxDrmSettings.FromEnvironment(name => name == variable ? value : null));
        Assert.Contains(variable, error.Message);
    }

    [Fact]
    public void Shutdown_reaches_a_non_desktop_controlled_lifetime()
    {
        var lifetime = new ControlledLifetime();
        AvnApplication.ShutdownLifetime(lifetime);
        Assert.True(lifetime.Stopped);
        Assert.Equal(0, lifetime.ExitCode);
    }

    private sealed class ControlledLifetime : IControlledApplicationLifetime
    {
        public bool Stopped { get; private set; }
        public int ExitCode { get; private set; }
        public event EventHandler<ControlledApplicationLifetimeStartupEventArgs>? Startup { add { } remove { } }
        public event EventHandler<ControlledApplicationLifetimeExitEventArgs>? Exit { add { } remove { } }

        public void Shutdown(int exitCode = 0)
        {
            Stopped = true;
            ExitCode = exitCode;
        }
    }
}
