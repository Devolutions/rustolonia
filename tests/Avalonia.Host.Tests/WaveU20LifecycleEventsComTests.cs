using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU20LifecycleEventsComTests
{
    [Fact]
    public void Styled_element_lifecycle_members_cross()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateBorder(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnBorder>(projected);

        Assert.Equal(0, wrapper.GetIsInitialized(out var initialized));
        Assert.Equal(0, initialized);
        Assert.Equal(0, wrapper.GetParent(out var parent));
        Assert.Null(parent);
        Assert.Equal(0, wrapper.GetDataContext(out var dataContext));
        Assert.Equal(AvnVariant.TagNone, dataContext.Tag);

        Assert.Equal(0, wrapper.SetDataContext(AvnVariant.FromObject("model")));
        Assert.Equal(0, wrapper.GetDataContext(out dataContext));
        Assert.Equal("model", (string?)dataContext.ToObject());

        Assert.Equal(0, wrapper.AdviseInitialized(new LifecycleHandler(), out var initSubscription));
        Assert.Equal(0, wrapper.AdviseAttachedToLogicalTree(new TreeHandler(), out var attachSubscription));
        Assert.Equal(0, wrapper.UnadviseInitialized(initSubscription));
        Assert.Equal(0, wrapper.UnadviseAttachedToLogicalTree(attachSubscription));
    }

    [Fact]
    public void Numeric_up_down_spinned_and_date_validation_carry_payloads()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateNumericUpDown(out var numeric));
        Assert.NotNull(numeric);
        Assert.Equal(0, factory.CreateCalendarDatePicker(out var picker));
        Assert.NotNull(picker);

        var numericWrapper = Assert.IsType<AvnNumericUpDown>(numeric);
        var pickerWrapper = Assert.IsType<AvnCalendarDatePicker>(picker);

        Assert.Equal(0, numericWrapper.AdviseSpinned(new SpinnedHandler(), out var spinned));
        Assert.Equal(0, numericWrapper.UnadviseSpinned(spinned));

        Assert.Equal(0, pickerWrapper.AdviseDateValidationError(new ValidationHandler(), out var validation));
        Assert.Equal(0, pickerWrapper.UnadviseDateValidationError(validation));
    }

    private sealed class LifecycleHandler : IAvnStyledElementInitializedHandler
    {
        public int Invoke() => 0;
    }

    private sealed class TreeHandler : IAvnStyledElementAttachedToLogicalTreeHandler
    {
        public int Invoke() => 0;
    }

    private sealed class SpinnedHandler : IAvnNumericUpDownSpinnedHandler
    {
        public int Invoke(int Direction, int UsingMouseWheel) => 0;
    }

    private sealed class ValidationHandler : IAvnCalendarDatePickerDateValidationErrorHandler
    {
        public int Invoke(string? Text, ref int ThrowException) => 0;
    }
}
