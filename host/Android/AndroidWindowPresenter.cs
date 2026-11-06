using System;
using System.Collections.Generic;
using Avalonia.Android;
using Avalonia.Controls;
using Avalonia.Platform;
using Avalonia.Threading;

namespace Avalonia.Host.Android;

/// <summary>
/// Maps Rustolonia's desktop window model onto one Android activity.
/// </summary>
/// <remarks>
/// Shown windows form a stack, like Android's own back stack. The activity
/// displays the top window's content: when a window is shown, its
/// <see cref="ContentControl.Content"/> is moved ("hoisted") into
/// <see cref="Root"/> and later content assignments to that window are moved
/// as well, so a control is never parented twice. The window's
/// <see cref="StyledElement.DataContext"/> flows to the root, and its title
/// becomes the activity title. Back closes the top window while more than one
/// is shown (its Closing handlers can cancel); closing the last window
/// finishes the activity. Sizes, positions, owners and decorations are ignored.
/// </remarks>
internal static class AndroidWindowPresenter
{
    private static readonly List<Window> s_stack = [];
    private static readonly Dictionary<Window, object?> s_content = [];
    private static ContentControl? s_root;
    private static AvaloniaActivity? s_activity;
    private static Control? s_mainView;
    private static bool s_hoisting;

    /// <summary>The control the activity hosts as its main view.</summary>
    internal static ContentControl Root => s_root ??= new ContentControl
    {
        HorizontalContentAlignment = Layout.HorizontalAlignment.Stretch,
        VerticalContentAlignment = Layout.VerticalAlignment.Stretch,
    };

    internal static Size? ViewportSize =>
        s_root is { } root && TopLevel.GetTopLevel(root) is not null ? root.Bounds.Size : null;

    internal static double RenderScaling =>
        s_root is not null && TopLevel.GetTopLevel(s_root) is { } topLevel
            ? topLevel.RenderScaling
            : global::Android.App.Application.Context.Resources?.DisplayMetrics?.Density ?? 1;

    internal static object? TryGetActivityFeature(Type featureType) =>
        s_root is not null && TopLevel.GetTopLevel(s_root)?.PlatformImpl is { } impl
            ? impl.TryGetFeature(featureType)
            : null;

    internal static long ZOrder(IWindowImpl impl)
    {
        for (var index = 0; index < s_stack.Count; index++)
        {
            if (ReferenceEquals(s_stack[index].PlatformImpl, impl))
                return index + 1;
        }
        return 0;
    }

    internal static void AttachActivity(AvaloniaActivity activity)
    {
        if (ReferenceEquals(s_activity, activity))
            return;
        DetachActivity(s_activity);
        s_activity = activity;
        activity.BackRequested += OnBackRequested;
        UpdateTitle();
    }

    internal static void DetachActivity(AvaloniaActivity? activity)
    {
        if (activity is null || !ReferenceEquals(s_activity, activity))
            return;
        activity.BackRequested -= OnBackRequested;
        s_activity = null;
    }

    internal static void Push(Window window)
    {
        Dispatcher.UIThread.VerifyAccess();
        if (s_stack.Contains(window))
            return;
        s_stack.Add(window);
        s_content[window] = TakeContent(window);
        window.PropertyChanged += OnWindowPropertyChanged;
        Present();
    }

    internal static void Remove(Window window)
    {
        if (!s_stack.Remove(window))
            return;
        window.PropertyChanged -= OnWindowPropertyChanged;
        s_content.Remove(window, out var content);
        if (ReferenceEquals(Root.Content, content))
            Root.Content = null;
        // Hand the content back so a reopened or inspected window still owns it.
        if (content is not null)
            Hoist(() => window.Content = content);
        if (s_stack.Count == 0 && s_mainView is null)
        {
            Root.Content = null;
            s_activity?.Finish();
            return;
        }
        Present();
    }

    /// <summary>
    /// <c>IAvnApplication.SetMainView</c>: the single-view root, shown
    /// whenever no window is. Closing the last window then returns to it
    /// instead of finishing the activity.
    /// </summary>
    internal static void SetMainView(Control? view)
    {
        Dispatcher.UIThread.VerifyAccess();
        s_mainView = view;
        Present();
    }

    /// <summary><c>IAvnApplication.Shutdown</c>: close every window, then the task.</summary>
    internal static void Shutdown()
    {
        for (var index = s_stack.Count - 1; index >= 0; index--)
        {
            if (index < s_stack.Count)
                s_stack[index].Close();
        }
        s_activity?.FinishAffinity();
    }

    internal static void TitleChanged(Window? window)
    {
        if (window is not null && s_stack.Count > 0 && ReferenceEquals(s_stack[^1], window))
            UpdateTitle();
    }

    private static object? TakeContent(Window window)
    {
        var content = window.Content;
        if (content is not null)
            Hoist(() => window.Content = null);
        return content;
    }

    private static void OnWindowPropertyChanged(object? sender, AvaloniaPropertyChangedEventArgs e)
    {
        if (s_hoisting || sender is not Window window)
            return;
        if (e.Property == ContentControl.ContentProperty)
        {
            s_content[window] = TakeContent(window);
            if (ReferenceEquals(s_stack[^1], window))
                Present();
        }
        else if (e.Property == StyledElement.DataContextProperty && ReferenceEquals(s_stack[^1], window))
        {
            Root.DataContext = window.DataContext;
        }
    }

    private static void Present()
    {
        var top = s_stack.Count > 0 ? s_stack[^1] : null;
        var content = top is null ? s_mainView : s_content.GetValueOrDefault(top);
        if (!ReferenceEquals(Root.Content, content))
        {
            // Detach first: the same control may be moving between windows.
            Root.Content = null;
            Root.Content = content;
        }
        Root.DataContext = top?.DataContext;
        UpdateTitle();
    }

    private static void UpdateTitle()
    {
        if (s_activity is { } activity && s_stack.Count > 0 && s_stack[^1].Title is { } title)
            activity.Title = title;
    }

    private static void Hoist(Action change)
    {
        s_hoisting = true;
        try
        {
            change();
        }
        finally
        {
            s_hoisting = false;
        }
    }

    private static void OnBackRequested(object? sender, AndroidBackRequestedEventArgs e)
    {
        // Back closes the top window unless it is the last screen left.
        if (e.Handled || s_stack.Count <= (s_mainView is null ? 1 : 0))
            return;
        e.Handled = true;
        s_stack[^1].Close();
    }
}
