using Avalonia.Controls.Notifications;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU26NotificationComTests
{
    [Fact]
    public void Show_and_close_accept_a_foreign_notification()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindowNotificationManager(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnWindowNotificationManager>(projected);
        var notification = new ForeignNotification();

        // The INotification-typed overloads cross through the new kind.
        Assert.Equal(0, wrapper.ShowWithINotification(notification));
        Assert.Equal(0, wrapper.CloseWithINotification(notification));
    }

    [Fact]
    public void The_wrapper_reads_through_the_foreign_notification()
    {
        var notification = new ForeignNotification();
        var managed = AvnNotification.ToNotification(notification)!;
        Assert.Equal("Title", managed.Title);
        Assert.Equal("Message", managed.Message);
        Assert.Equal(NotificationType.Information, managed.Type);
        // The foreign notification reports no handlers; the wrapper surfaces null.
        Assert.Null(managed.OnClick);
        Assert.Null(managed.OnClose);
    }

    private sealed class ForeignNotification : IAvnNotification
    {
        public int GetTitle(out string? value)
        {
            value = "Title";
            return 0;
        }

        public int GetMessage(out string? value)
        {
            value = "Message";
            return 0;
        }

        public int GetType(out int value)
        {
            value = 0;
            return 0;
        }

        public int GetExpiration(out long value)
        {
            value = 0;
            return 0;
        }

        public int GetOnClick(out IAvnNotificationActionHandler? value)
        {
            value = null;
            return 0;
        }

        public int GetOnClose(out IAvnNotificationActionHandler? value)
        {
            value = null;
            return 0;
        }
    }
}
