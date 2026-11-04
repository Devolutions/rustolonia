using System;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public class FileDragTests
{
    [Fact]
    public void File_drag_is_additive_and_stale_tokens_fail()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        Assert.Equal(Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA6"), typeof(IAvnApplication12).GUID);
        Assert.Equal(3, typeof(IAvnApplication12).GetMethods().Length);
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
