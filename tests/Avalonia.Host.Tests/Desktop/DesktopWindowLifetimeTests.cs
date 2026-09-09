using System;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Host.Desktop;
using Avalonia.Threading;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// A picker is parented to a window, so the window's lifetime bounds it. These
/// cover the real <c>Window.Closed</c> wiring the ABI installs, using a real
/// window rather than a stand-in signal.
/// </summary>
public class DesktopWindowLifetimeTests
{
    [Fact]
    public void Closing_a_window_notifies_the_observer_exactly_once()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var window = new Window();
        var notifications = 0;

        using (AvnApplication.ObserveClosed(window, () => notifications++))
        {
            window.Show();
            window.Close();
        }

        Assert.Equal(1, notifications);
    }

    [Fact]
    public void Disposing_the_observer_detaches_it_from_the_window()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var window = new Window();
        var notifications = 0;

        AvnApplication.ObserveClosed(window, () => notifications++).Dispose();
        window.Show();
        window.Close();

        Assert.Equal(0, notifications);
    }

    [Fact]
    public void Disposing_the_observer_twice_is_harmless()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var window = new Window();
        var observer = AvnApplication.ObserveClosed(window, static () => { });

        observer.Dispose();
        observer.Dispose();
    }

    [Fact]
    public void A_real_window_closing_aborts_the_picker_it_owns()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var window = new Window();
        window.Show();

        var provider = new FakeStorageProvider { Gate = new TaskCompletionSource<bool>() };
        var picker = DesktopFilePickers.RunWhileOpenAsync(
            closed => AvnApplication.ObserveClosed(window, closed),
            cancellation => DesktopFilePickers.OpenFilesAsync(
                provider,
                new DesktopFilePickerRequest(),
                cancellation),
            CancellationToken.None);

        Assert.False(picker.IsCompleted);
        window.Close();

        // The continuation resumes through the dispatcher, and the test drives
        // it explicitly rather than awaiting: awaiting under Avalonia's
        // synchronization context would resume on a different thread once the
        // unit-test application scope is torn down.
        Pump(picker);

        Assert.True(picker.IsCanceled || picker.Exception?.InnerException is OperationCanceledException);
        Assert.Equal(1, provider.OpenCallCount);
        Assert.False(provider.Gate!.Task.IsCompleted, "the dialog was never dismissed");
    }

    private static void Pump(Task task)
    {
        for (var i = 0; i < 100 && !task.IsCompleted; i++)
        {
            Dispatcher.UIThread.RunJobs();
            if (!task.IsCompleted)
                Thread.Sleep(10);
        }

        Assert.True(task.IsCompleted, "the picker never completed");
    }
}
