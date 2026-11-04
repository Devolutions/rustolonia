using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class MainViewTests
{
    [Fact]
    public void Set_main_view_is_not_implemented_without_single_view_lifetime()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        Assert.Equal(Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA8"), typeof(IAvnApplication13).GUID);
        Assert.Single(typeof(IAvnApplication13).GetMethods());
        var application = new AvnApplication();
        Assert.Equal(HResults.E_NOTIMPL, application.SetMainView(null));
        Assert.Equal(HResults.E_NOTIMPL, application.SetMainView(new AvnButton(new Button())));
    }
}
