using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU34ShowAtPointerComTests
{
    [Fact]
    public void Flyout_show_at_pointer_crosses_without_pointer_state()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var target));
        Assert.Equal(0, factory.CreateMenuFlyout(out var flyout));
        Assert.NotNull(flyout);

        var wrapper = Assert.IsType<AvnMenuFlyout>(flyout);

        // The pointer-tracking overload takes only a flag: the placement mode
        // switches to Pointer and the host needs no pointer-event payload.
        Assert.Equal(0, wrapper.ShowAtWithControlAndBoolean(
            Assert.IsType<AvnButton>(target),
            1));

        var value = Assert.IsType<MenuFlyout>(
            typeof(AvnMenuFlyout)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.True(value.IsOpen);

        // The plain Hide still closes.
        Assert.Equal(0, wrapper.Hide());
        Assert.False(value.IsOpen);
    }
}
