using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class FileDragTests
{
    [Fact]
    public void File_drag_stale_tokens_fail()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        Assert.Equal(2, typeof(IAvnFileDragHandler).GetMethods().Length);
        var application = new AvnApplication();
        var expected = OperatingSystem.IsMacOS() || OperatingSystem.IsLinux() ? HResults.E_INVALIDARG : HResults.E_NOTIMPL;
        Assert.Equal(expected, application.CompleteFileDrag(42, "[]"));
        Assert.Equal(expected, application.CancelFileDrag(42));
        if (OperatingSystem.IsMacOS() || OperatingSystem.IsLinux())
        {
            Assert.Equal(HResults.E_POINTER, application.SetFileDragHandler(null, null));
            Assert.Equal(HResults.S_OK, application.SetFileDragHandler(new AvnButton(new Button()), null));
        }
    }
}
