using Avalonia.Host.Com;
using Avalonia.Threading;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class TextBindingEventTests
{
    [Fact]
    public void Text_changing_is_synchronous_and_text_changed_is_dispatched()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTextBox(out var projected));
        var textBox = Assert.IsType<AvnTextBox>(projected);
        var changing = new TextChangingHandler();
        var changed = new TextChangedHandler();
        Assert.Equal(0, textBox.AdviseTextChanging(changing, out var changingSubscription));
        Assert.Equal(0, textBox.AdviseTextChanged(changed, out var changedSubscription));

        Assert.Equal(0, textBox.SetText("42"));
        Assert.Equal(1, changing.Count);
        Assert.Equal(0, changed.Count);

        Dispatcher.UIThread.RunJobs();
        Assert.Equal(1, changed.Count);
        Assert.Equal(0, textBox.UnadviseTextChanging(changingSubscription));
        Assert.Equal(0, textBox.UnadviseTextChanged(changedSubscription));
    }

    private sealed class TextChangingHandler : IAvnTextBoxTextChangingHandler
    {
        public int Count;

        public int Invoke()
        {
            Count++;
            return 0;
        }
    }

    private sealed class TextChangedHandler : IAvnTextBoxTextChangedHandler
    {
        public int Count;

        public int Invoke()
        {
            Count++;
            return 0;
        }
    }
}
