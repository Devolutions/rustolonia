#if RUSTOLONIA_IOS_SMOKE
using System;
using System.IO;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Avalonia.VisualTree;

namespace Rustolonia.IosSample;

internal static class IosSmokeTest
{
    [DllImport("__Internal", EntryPoint = "rustolonia_ios_stop")]
    private static extern int StopRust();

    internal static void Begin() => Run(() =>
    {
        var application = Application.Current as HostApplication
            ?? throw new InvalidOperationException("The Avalonia host is not initialized.");
        var lifetime = application.ApplicationLifetime as ISingleViewApplicationLifetime
            ?? throw new InvalidOperationException("The single-view lifetime is not available.");
        var root = GetRoot(lifetime);
        WhenRendered(root, () =>
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            var (label, button) = ClickCounter(root);
            if (StopRust() < 0)
                throw new InvalidOperationException("Rust single-view shutdown failed.");
            if (lifetime.MainView is not null)
                throw new InvalidOperationException("The main view was not detached.");
            button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            if (label.Text != "Taps: 1")
                throw new InvalidOperationException("The stopped view still receives Rust callbacks.");
            var stoppedKey = ResetKey();
            button.RaiseEvent(stoppedKey);
            if (stoppedKey.Handled || label.Text != "Taps: 1")
                throw new InvalidOperationException("The stopped view still receives Rust key callbacks.");

            application.OnFrameworkInitializationCompleted();
            var restarted = GetRoot(lifetime);
            WhenRendered(restarted, () =>
            {
                ClickCounter(restarted);
                WriteResult("ok");
            });
        });
    });

    private static StackPanel GetRoot(ISingleViewApplicationLifetime lifetime) =>
        lifetime.MainView as StackPanel
        ?? throw new InvalidOperationException("The Rust root view was not installed.");

    private static void WhenRendered(StackPanel root, Action action)
    {
        void CheckLayout() => DispatcherTimer.RunOnce(() => Run(() =>
        {
            if (!root.IsAttachedToVisualTree() || root.Bounds.Width <= 0 || root.Bounds.Height <= 0)
                throw new InvalidOperationException("The Rust root view was not laid out.");
            action();
        }), TimeSpan.FromSeconds(1));

        if (root.IsAttachedToVisualTree())
            CheckLayout();
        else
            root.AttachedToVisualTree += (_, _) => CheckLayout();
    }

    private static (TextBlock Label, Button Button) ClickCounter(StackPanel root)
    {
        if (root.Children.Count != 4 ||
            root.Children[1] is not TextBlock label ||
            root.Children[2] is not Button button ||
            label.Text != "Taps: 0")
            throw new InvalidOperationException("The counter did not render its initial state.");

        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        if (label.Text != "Taps: 1" || button.Content is not TextBlock { Text: "Tap again" })
            throw new InvalidOperationException("The Rust click callback did not update the view.");

        var key = ResetKey();
        button.RaiseEvent(key);
        if (!key.Handled || label.Text != "Taps: 0")
            throw new InvalidOperationException("The Rust key callback did not reset the view.");
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        if (label.Text != "Taps: 1")
            throw new InvalidOperationException("The Rust counter did not restart after reset.");
        var shiftedKey = ResetKey("R");
        button.RaiseEvent(shiftedKey);
        if (!shiftedKey.Handled || label.Text != "Taps: 0")
            throw new InvalidOperationException("The Rust key callback did not handle uppercase R.");
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        if (label.Text != "Taps: 1")
            throw new InvalidOperationException("The Rust counter did not restart after uppercase reset.");
        return (label, button);
    }

    private static KeyEventArgs ResetKey(string symbol = "r") => new()
    {
        RoutedEvent = InputElement.KeyDownEvent,
        Key = Key.R,
        KeySymbol = symbol,
    };

    private static void Run(Action action)
    {
        try
        {
            action();
        }
        catch (Exception error)
        {
            WriteResult($"FAILED: {error}");
        }
    }

    private static void WriteResult(string result)
    {
        Console.WriteLine($"Rustolonia iOS smoke test: {result}");
        File.WriteAllText(Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
            "rustolonia-ios-smoke.txt"), result);
    }
}
#endif
