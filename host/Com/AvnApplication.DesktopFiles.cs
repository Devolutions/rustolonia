using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Desktop;
using Avalonia.Input;
using Avalonia.Platform.Storage;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

/// <summary>
/// Stage 29 desktop file integration: pickers, incoming drag-and-drop and
/// startup/open-with activation, all routed through Avalonia's platform-neutral
/// abstractions (<see cref="IStorageProvider"/>, <see cref="DragDrop"/> and the
/// desktop lifetime). No platform dialog API is used.
/// </summary>
public partial class AvnApplication : IAvnApplication3
{
    private readonly List<string> _startupArguments = new();
    private readonly AvnFileDropRegistry _fileDrops = new();
    private readonly Dictionary<long, (IAvnActivationHandler Handler, Action Unsubscribe)>
        _activationSubscriptions = new();
    private long _nextActivationSubscriptionId;

    public int CreatePickerOptions(out IAvnFilePickerOptions? options)
    {
        options = null;
        try
        {
            options = new AvnFilePickerOptions();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int GetStorageCapabilities(IAvnWindow? window, out int capabilities)
    {
        capabilities = 0;
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            capabilities = (int)DesktopFilePickers.GetCapabilities(ResolveStorageProvider(window));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartOpenFilePicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId) =>
        StartPicker(
            window,
            options,
            completion,
            static (provider, request, cancellation) =>
                DesktopFilePickers.OpenFilesAsync(provider, request, cancellation),
            out operationId);

    public int StartOpenFolderPicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId) =>
        StartPicker(
            window,
            options,
            completion,
            static (provider, request, cancellation) =>
                DesktopFilePickers.OpenFoldersAsync(provider, request, cancellation),
            out operationId);

    public int StartSaveFilePicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        out long operationId) =>
        StartPicker(
            window,
            options,
            completion,
            static (provider, request, cancellation) =>
                DesktopFilePickers.SaveFileAsync(provider, request, cancellation),
            out operationId);

    public int SubscribeFileDrop(
        IAvnControl? target,
        int acceptedEffects,
        IAvnFileDropHandler? handler,
        out long subscriptionId)
    {
        subscriptionId = 0;
        if (target is null || handler is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var control = (Control?)ProjectionRuntime.Unwrap(target)
                ?? throw new ObjectDisposedException(nameof(target));
            return _fileDrops.Subscribe(
                control,
                (DragDropEffects)acceptedEffects,
                handler,
                out subscriptionId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int UnsubscribeFileDrop(long subscriptionId)
    {
        try
        {
            return _fileDrops.Unsubscribe(subscriptionId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int ClearStartupArguments()
    {
        lock (_startupArguments)
            _startupArguments.Clear();
        return HResults.S_OK;
    }

    public int AddStartupArgument(string? value)
    {
        if (value is null)
            return HResults.E_POINTER;
        lock (_startupArguments)
            _startupArguments.Add(value);
        return HResults.S_OK;
    }

    public int GetStartupArgumentCount(out int count)
    {
        lock (_startupArguments)
            count = _startupArguments.Count;
        return HResults.S_OK;
    }

    public int GetStartupArgument(int index, out string? value)
    {
        value = null;
        lock (_startupArguments)
        {
            if (index < 0 || index >= _startupArguments.Count)
                return HResults.E_INVALIDARG;
            value = _startupArguments[index];
        }

        return HResults.S_OK;
    }

    public int GetActivationItems(out IAvnStorageItemList? items)
    {
        items = null;
        try
        {
            items = new AvnStorageItemList(
                StorageItemSnapshot.FromActivationArguments(SnapshotStartupArguments()));
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int AdviseActivation(IAvnActivationHandler? handler, out long subscriptionId)
    {
        subscriptionId = 0;
        if (handler is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var id = Interlocked.Increment(ref _nextActivationSubscriptionId);

            // Only some platforms expose post-startup activation (macOS "open
            // with" while running, protocol activation, dock reopen). Where the
            // feature is absent the subscription is still valid and simply never
            // fires, so consumers do not need platform branches.
            var lifetime = Application.Current?.TryGetFeature<IActivatableLifetime>();
            if (lifetime is null)
            {
                _activationSubscriptions.Add(id, (handler, static () => { }));
                subscriptionId = id;
                return HResults.S_OK;
            }

            var callback = new EventHandler<ActivatedEventArgs>((_, e) => OnActivated(handler, e));
            lifetime.Activated += callback;
            _activationSubscriptions.Add(id, (handler, () => lifetime.Activated -= callback));
            subscriptionId = id;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int UnadviseActivation(long subscriptionId)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (!_activationSubscriptions.Remove(subscriptionId, out var subscription))
                return HResults.E_INVALIDARG;
            subscription.Unsubscribe();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    internal string[] SnapshotStartupArguments()
    {
        lock (_startupArguments)
            return _startupArguments.ToArray();
    }

    private void ReleaseDesktopFileIntegration()
    {
        // The dispatcher loop has stopped, so anything still pending has to be
        // completed here rather than merely cancelled: a cancelled operation
        // delivers its completion through a dispatcher continuation that can no
        // longer run. `Shutdown` is also only one of the ways `Run` returns (the
        // last window closing is the other), and a consumer's future must not
        // stay pending forever because the application simply exited.
        _asyncOperations.AbortAll();
        foreach (var subscription in _activationSubscriptions.Values)
            subscription.Unsubscribe();
        _activationSubscriptions.Clear();
        _fileDrops.Clear();
    }

    private static void OnActivated(IAvnActivationHandler handler, ActivatedEventArgs e)
    {
        try
        {
            var items = e is FileActivatedEventArgs files
                ? StorageItemSnapshot.FromStorageItems(files.Files)
                : e is ProtocolActivatedEventArgs protocol
                    ? StorageItemSnapshot.FromActivationArguments(
                        new[] { protocol.Uri.ToString() })
                    : Array.Empty<StorageItemSnapshot>();
            var hr = handler.OnActivated((int)e.Kind, new AvnStorageItemList(items));
            if (hr < 0)
                Marshal.ThrowExceptionForHR(hr);
        }
        catch (Exception error)
        {
            _ = AbiError.Capture(error);
        }
    }

    private int StartPicker(
        IAvnWindow? window,
        IAvnFilePickerOptions? options,
        IAvnStorageCompletion? completion,
        Func<IStorageProvider, DesktopFilePickerRequest, CancellationToken, Task<DesktopPickerResult>> run,
        out long operationId)
    {
        operationId = 0;
        if (window is null || options is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();

            // Both the provider and the request are resolved now, on the UI
            // thread, and captured by value: a consumer that keeps mutating the
            // options builder cannot change an in-flight dialog. The dialog is
            // additionally tied to the window it is parented to, so closing that
            // window aborts the operation instead of leaving the consumer's
            // future pending forever.
            var provider = ResolveStorageProvider(window, out var managedWindow);
            var request = ((AvnFilePickerOptions)options).Snapshot();
            return _asyncOperations.StartStorage(
                completion,
                cancellation => DesktopFilePickers.RunWhileOpenAsync(
                    closed => ObserveClosed(managedWindow, closed),
                    linked => run(provider, request, linked),
                    cancellation),
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    /// <summary>
    /// Subscribes <paramref name="closed"/> to the window's
    /// <see cref="Window.Closed"/> event and returns the unsubscription.
    /// </summary>
    internal static IDisposable ObserveClosed(Window window, Action closed)
    {
        void OnClosed(object? sender, EventArgs e) => closed();
        window.Closed += OnClosed;
        return new Unsubscribe(() => window.Closed -= OnClosed);
    }

    private sealed class Unsubscribe(Action dispose) : IDisposable
    {
        private Action? _dispose = dispose;

        public void Dispose() => Interlocked.Exchange(ref _dispose, null)?.Invoke();
    }

    private static IStorageProvider ResolveStorageProvider(IAvnWindow window) =>
        ResolveStorageProvider(window, out _);

    private static IStorageProvider ResolveStorageProvider(
        IAvnWindow window,
        out Window managedWindow)
    {
        managedWindow = (Window?)ProjectionRuntime.Unwrap(window)
            ?? throw new ObjectDisposedException(nameof(window));
        return TopLevel.GetTopLevel(managedWindow)?.StorageProvider
            ?? throw new NotSupportedException("The top-level has no storage provider.");
    }
}
