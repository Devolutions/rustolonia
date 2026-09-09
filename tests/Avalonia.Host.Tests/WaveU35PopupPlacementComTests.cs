using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Controls.Primitives.PopupPositioning;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU35PopupPlacementComTests
{
    [Fact]
    public void Popup_custom_placement_round_trips_through_the_callback()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePopup(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnPopup>(projected);

        // A foreign CCW converts into the managed delegate and positions the popup:
        // the host passes the geometry and reads the mutated placement back.
        var foreign = new ForeignPlacement();
        Assert.Equal(0, wrapper.SetCustomPopupPlacementCallback(foreign));

        var value = Assert.IsType<Popup>(
            typeof(AvnPopup)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        var managed = value.CustomPopupPlacementCallback;
        Assert.NotNull(managed);

        var size = new Avalonia.Size(100, 50);
        var anchor = new Avalonia.Rect(10, 20, 30, 40);
        var parameters = new CustomPopupPlacement(size, null!)
        {
            AnchorRectangle = anchor,
        };
        managed!(parameters);
        Assert.True(foreign.Invoked);
        Assert.Equal(12.5, parameters.Offset.X);
        Assert.Equal(-7.25, parameters.Offset.Y);
        Assert.Equal(PopupAnchor.Bottom, parameters.Anchor);
        Assert.Equal(PopupGravity.Top, parameters.Gravity);

        // Null clears the callback.
        Assert.Equal(0, wrapper.SetCustomPopupPlacementCallback(null));
        Assert.Null(value.CustomPopupPlacementCallback);
    }

    private sealed class ForeignPlacement : IAvnPopupPlacementCallback
    {
        public bool Invoked;

        public int Invoke(
            double popupWidth,
            double popupHeight,
            double anchorX,
            double anchorY,
            double anchorWidth,
            double anchorHeight,
            out double offsetX,
            out double offsetY,
            out int anchor,
            out int gravity,
            out int constraintAdjustment)
        {
            Invoked = true;
            Assert.Equal(100, popupWidth);
            Assert.Equal(50, popupHeight);
            Assert.Equal(10, anchorX);
            Assert.Equal(20, anchorY);
            offsetX = 12.5;
            offsetY = -7.25;
            anchor = (int)PopupAnchor.Bottom;
            gravity = (int)PopupGravity.Top;
            constraintAdjustment = 0;
            return 0;
        }
    }
}
