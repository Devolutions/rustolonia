using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class ApplicationNameTests
{
    [Fact]
    public void Name_crosses_the_COM_boundary_as_utf16()
    {
        var host = new AvnApplication();
        var wrappers = new StrategyBasedComWrappers();
        var unknown = wrappers.GetOrCreateComInterfaceForObject(host, CreateComInterfaceFlags.None);
        try
        {
            var application = (IAvnApplication14)wrappers.GetOrCreateObjectForComInstance(
                unknown, CreateObjectFlags.None);
            const string name = "Jiffshot \ud83d\udcf7";
            Assert.Equal(HResults.S_OK, application.SetApplicationName(name));
            Assert.Equal(name, host.CreateApplication().Name);
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    [Fact]
    public void Naming_capability_is_additive()
    {
        Assert.Equal(Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA9"),
            typeof(IAvnApplication14).GUID);
        Assert.Equal(nameof(IAvnApplication14.SetApplicationName),
            Assert.Single(typeof(IAvnApplication14).GetMethods()).Name);
        Assert.IsAssignableFrom<IAvnApplication14>(new AvnApplication());
    }

    [Fact]
    public void Unconfigured_name_preserves_the_framework_default()
    {
        Assert.Equal(new HostApplication().Name, new AvnApplication().CreateApplication().Name);
    }

    [Theory]
    [InlineData("Jiffshot")]
    [InlineData(" Jiffshot \ud83d\udcf7 ")]
    public void Name_is_available_before_application_initialization(string name)
    {
        var host = new AvnApplication();
        Assert.Equal(HResults.S_OK, host.SetApplicationName("Previous name"));
        Assert.Equal(HResults.S_OK, host.SetApplicationName(name));
        var application = host.CreateApplication();
        Assert.Equal(name, application.Name);
        Assert.Null(Application.Current);
        Assert.True(host.SetApplicationName("Too late") < 0);
        Assert.Contains("before Avalonia initialization", AbiError.Take());
        Assert.Equal(name, application.Name);
    }

    [Theory]
    [InlineData("")]
    [InlineData(" \t\n")]
    [InlineData("\u2003")]
    [InlineData("Jiff\0shot")]
    public void Invalid_name_does_not_replace_a_staged_name(string name)
    {
        var host = new AvnApplication();
        Assert.Equal(HResults.S_OK, host.SetApplicationName("Jiffshot"));
        Assert.Equal(HResults.E_INVALIDARG, host.SetApplicationName(name));
        Assert.Equal(HResults.E_POINTER, host.SetApplicationName(null));
        Assert.Equal("Jiffshot", host.CreateApplication().Name);
    }

    [Fact]
    public void Already_initialized_platform_rejects_name_changes()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var previous = Application.Current!.Name;
        Assert.True(new AvnApplication().SetApplicationName("Too late") < 0);
        Assert.Contains("before Avalonia initialization", AbiError.Take());
        Assert.Equal(previous, Application.Current.Name);
    }
}
