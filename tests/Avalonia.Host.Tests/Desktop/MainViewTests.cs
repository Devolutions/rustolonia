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
        var application = new AvnApplication();
        Assert.Equal(HResults.E_NOTIMPL, application.SetMainView(null));
        Assert.Equal(HResults.E_NOTIMPL, application.SetMainView(new AvnButton(new Button())));
    }
}
