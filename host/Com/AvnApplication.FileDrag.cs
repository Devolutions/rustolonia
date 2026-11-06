using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Host.Ownership;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using Avalonia.VisualTree;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
    private readonly Dictionary<Control, FileDragRegistration> _fileDrags = new();
    private static readonly Dictionary<Control, FileDragRegistration> ActiveFileDrags = new();
    private static int FileDragCallbackDepth;
    private readonly Dictionary<long, FileDragRegistration> _fileDragTokens = new();
    private long _nextFileDragToken;
    private static bool SupportsFileDrag => OperatingSystem.IsMacOS() || OperatingSystem.IsLinux();

    public int SetFileDragHandler(IAvnControl? target, IAvnFileDragHandler? handler)
    {
        if (!SupportsFileDrag)
            return HResults.E_NOTIMPL;
        if (target is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (FileDragCallbackDepth != 0)
                return HResults.E_INVALIDARG;
            var control = (Control?)ProjectionRuntime.Unwrap(target)
                ?? throw new ObjectDisposedException(nameof(target));
            _fileDrags.TryGetValue(control, out var old);
            if (old is { Started: true } or { Notifying: true })
                throw new InvalidOperationException("A native drag or its callback is still active.");
            Window? owner = null;
            if (handler is not null)
            {
                foreach (var (registered, registration) in ActiveFileDrags)
                    if (registration != old
                        && (registered == control || registered.IsVisualAncestorOf(control) || control.IsVisualAncestorOf(registered)))
                        throw new InvalidOperationException("Drag targets must not overlap, including across auxiliary applications.");
                if (OperatingSystem.IsMacOS() && DragNative.Supports() != 1)
                    throw new NotSupportedException("The native companion cannot retain originating drag events.");
                owner = TopLevel.GetTopLevel(control) as Window
                    ?? throw new InvalidOperationException("Install the drag handler on a mounted window control.");
            }
            old?.Dispose();
            if (handler is not null && owner is not null)
            {
                var registration = new FileDragRegistration(this, owner, control, handler);
                _fileDrags.Add(control, registration);
                ActiveFileDrags.Add(control, registration);
            }
            return HResults.S_OK;
        }
        catch (Exception error) { return AbiError.Capture(error); }
    }

    public int CompleteFileDrag(long token, string? fileUrlsJson)
    {
        if (!SupportsFileDrag)
            return HResults.E_NOTIMPL;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (FileDragCallbackDepth != 0)
                return HResults.E_INVALIDARG;
            if (!_fileDragTokens.TryGetValue(token, out var drag) || drag.Started || drag.Resolving || drag.Notifying || drag.Press is null)
                return HResults.E_INVALIDARG;
            if (fileUrlsJson is null)
                return HResults.E_POINTER;
            var urls = JsonSerializer.Deserialize(fileUrlsJson, FileDragJsonContext.Default.StringArray)
                ?? throw new ArgumentException("Expected a JSON array of file URLs.");
            if (urls.Length == 0)
                throw new ArgumentException("At least one file URL is required.");
            foreach (var text in urls)
            {
                if (!Uri.TryCreate(text, UriKind.Absolute, out var uri) || !uri.IsFile
                    || !string.IsNullOrEmpty(uri.Host) && uri.Host != "localhost"
                    || uri.Query.Length != 0 || uri.Fragment.Length != 0 || !File.Exists(uri.LocalPath))
                    throw new ArgumentException("Every drag entry must be an existing local file URL.");
            }
            drag.ResolveAndStart(urls);
            return HResults.S_OK;
        }
        catch (Exception error) { return AbiError.Capture(error); }
    }

    public int CancelFileDrag(long token)
    {
        if (!SupportsFileDrag)
            return HResults.E_NOTIMPL;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (FileDragCallbackDepth != 0)
                return HResults.E_INVALIDARG;
            if (!_fileDragTokens.TryGetValue(token, out var drag) || drag.Started || drag.Notifying)
                return HResults.E_INVALIDARG;
            drag.Cancel();
            return HResults.S_OK;
        }
        catch (Exception error) { return AbiError.Capture(error); }
    }

    private sealed class FileDragRegistration : IDisposable
    {
        private readonly AvnApplication _app;
        private readonly Window _owner;
        private readonly Control _target;
        private readonly IAvnFileDragHandler _handler;
        private nint _event;
        private long _token;
        private bool _changingCapture;
        private bool _resolving;
        private bool _disposed;
        private (long Token, int Effect, int Status)? _deferredCompletion;
        public PointerPressedEventArgs? Press { get; private set; }
        public bool Started { get; private set; }
        public bool Resolving => _resolving;
        public bool Notifying { get; private set; }

        public FileDragRegistration(AvnApplication app, Window owner, Control target, IAvnFileDragHandler handler)
        {
            _app = app; _owner = owner; _target = target; _handler = handler;
            owner.AddHandler(InputElement.PointerPressedEvent, Pressed, RoutingStrategies.Tunnel, true);
            owner.AddHandler(InputElement.PointerMovedEvent, Moved, RoutingStrategies.Tunnel, true);
            owner.AddHandler(InputElement.PointerReleasedEvent, Released, RoutingStrategies.Tunnel, true);
            owner.AddHandler(InputElement.PointerCaptureLostEvent, CaptureLost, RoutingStrategies.Bubble, true);
            owner.AddHandler(InputElement.KeyDownEvent, KeyDown, RoutingStrategies.Tunnel, true);
            target.DetachedFromVisualTree += Detached;
            owner.Closed += Closed;
        }

        private void Pressed(object? sender, PointerPressedEventArgs args)
        {
            if (_disposed || Started || _resolving || args.Source is not Visual source
                || source != _target && !_target.IsVisualAncestorOf(source)
                || !args.GetCurrentPoint(_target).Properties.IsLeftButtonPressed)
                return;
            Cancel();
            if (OperatingSystem.IsMacOS())
            {
                var handle = _owner.TryGetPlatformHandle();
                if (handle?.HandleDescriptor != "NSWindow")
                {
                    _ = AbiError.Capture(new InvalidOperationException("No native NSWindow for the drag owner."));
                    return;
                }
                _event = DragNative.Retain(MacOS.ObjC.GetWindowNumber(handle.Handle));
                if (_event == 0)
                {
                    _ = AbiError.Capture(new InvalidOperationException("File dragging requires an originating AppKit left mouse-down event."));
                    return;
                }
            }
            Press = args;
        }

        private void Moved(object? sender, PointerEventArgs args)
        {
            if (Press is null || Started || args.Pointer != Press.Pointer)
                return;
            if (_token != 0) { args.Handled = true; return; }
            var delta = args.GetPosition(_owner) - Press.GetPosition(_owner);
            if (delta.X * delta.X + delta.Y * delta.Y < 25)
                return;
            args.Handled = true;
            _token = checked(++_app._nextFileDragToken);
            _app._fileDragTokens.Add(_token, this);
            // Reset Button's pressed state through Avalonia's capture-lost path;
            // recapture only to receive release/cancellation while preparing files.
            _changingCapture = true;
            try { Press.Pointer.Capture(null); Press.Pointer.Capture(_target); }
            finally { _changingCapture = false; }
            Notifying = true;
            FileDragCallbackDepth++;
            int hr;
            try { hr = _handler.OnPreparing(_token); }
            finally
            {
                Notifying = false;
                FileDragCallbackDepth--;
                if (_deferredCompletion is { } completion)
                {
                    _deferredCompletion = null;
                    NotifyCompleted(completion.Token, completion.Effect, completion.Status);
                }
            }
            if (hr < 0) { _ = AbiError.Capture(Marshal.GetExceptionForHR(hr)!); Cancel(hr); }
        }

        private void Released(object? sender, PointerReleasedEventArgs args)
        {
            if (Press?.Pointer != args.Pointer || Started)
                return;
            if (_token != 0)
                args.Handled = true;
            Cancel();
        }
        private void CaptureLost(object? sender, PointerCaptureLostEventArgs args)
        {
            if (!_changingCapture && !Started && Press?.Pointer == args.Pointer && args.Pointer.Captured != _target)
                Cancel();
        }
        private void KeyDown(object? sender, KeyEventArgs args)
        {
            if (args.Key == Key.Escape && _token != 0 && !Started) { args.Handled = true; Cancel(); }
        }
        private void Detached(object? sender, VisualTreeAttachmentEventArgs args) => Dispose();
        private void Closed(object? sender, EventArgs args) => Dispose();

        public async void ResolveAndStart(string[] urls)
        {
            if (_resolving)
                throw new InvalidOperationException("This gesture already has a file payload being resolved.");
            _resolving = true;
            var token = _token;
            var data = new DataTransfer();
            try
            {
                foreach (var url in urls)
                {
                    var file = await (_owner.StorageProvider?.TryGetFileFromPathAsync(new Uri(url))
                        ?? throw new NotSupportedException("File dragging requires a storage provider."));
                    if (_token != token || Press is null || _disposed)
                        return;
                    if (file is null)
                        throw new IOException("A drag file could not be resolved.");
                    data.Add(DataTransferItem.CreateFile(file));
                }
                if (_token != token || Press is null || _disposed)
                    return;
                var press = Press;
                Started = true;
                System.Threading.Tasks.Task<DragDropEffects> task;
                if (OperatingSystem.IsMacOS())
                {
                    if (DragNative.Set(_event) != 1)
                        throw new InvalidOperationException("The originating AppKit event could not be selected.");
                    try { task = DragDrop.DoDragDropAsync(press, data, DragDropEffects.Copy); }
                    finally { DragNative.Set(0); }
                }
                else
                    task = DragDrop.DoDragDropAsync(press, data, DragDropEffects.Copy);
                var result = await task;
                Finish(result == DragDropEffects.Copy ? 1 : 0, HResults.S_OK);
            }
            catch (Exception error)
            {
                var hr = AbiError.Capture(error);
                Finish(0, hr);
            }
            finally { ((IDisposable)data).Dispose(); _resolving = false; }
        }

        public void Cancel(int status = HResults.S_OK)
        {
            if (Started)
                return; // AppKit owns an active session; completion, not caller cancellation, ends it.
            Finish(0, status);
        }

        private void Finish(int effect, int status)
        {
            var token = _token;
            _token = 0;
            if (token != 0) _app._fileDragTokens.Remove(token);
            var press = Press;
            Press = null;
            Started = false;
            if (_event != 0) { DragNative.Release(_event); _event = 0; }
            if (token != 0 && press?.Pointer.Captured == _target)
            {
                _changingCapture = true;
                try { press.Pointer.Capture(null); }
                finally { _changingCapture = false; }
            }
            if (_disposed)
                RemoveRegistration();
            if (token != 0)
            {
                if (Notifying)
                    _deferredCompletion = (token, effect, status);
                else
                    NotifyCompleted(token, effect, status);
            }
        }

        private void NotifyCompleted(long token, int effect, int status)
        {
            Notifying = true;
            FileDragCallbackDepth++;
            int hr;
            try { hr = _handler.OnCompleted(token, effect, status); }
            finally { Notifying = false; FileDragCallbackDepth--; }
            if (hr < 0) _ = AbiError.Capture(Marshal.GetExceptionForHR(hr)!);
        }

        public void Dispose()
        {
            if (_disposed) return;
            _disposed = true;
            _owner.RemoveHandler(InputElement.PointerPressedEvent, Pressed);
            _owner.RemoveHandler(InputElement.PointerMovedEvent, Moved);
            _owner.RemoveHandler(InputElement.PointerReleasedEvent, Released);
            _owner.RemoveHandler(InputElement.PointerCaptureLostEvent, CaptureLost);
            _owner.RemoveHandler(InputElement.KeyDownEvent, KeyDown);
            _target.DetachedFromVisualTree -= Detached;
            _owner.Closed -= Closed;
            Cancel();
            if (!Started)
                RemoveRegistration();
        }

        private void RemoveRegistration()
        {
            _app._fileDrags.Remove(_target);
            ActiveFileDrags.Remove(_target);
        }
    }

    private static class DragNative
    {
        [DllImport("libAvaloniaNative.dylib", EntryPoint = "avn_supports_retained_file_drag_event")]
        internal static extern int Supports();
        [DllImport("libAvaloniaNative.dylib", EntryPoint = "avn_retain_file_drag_event")]
        internal static extern nint Retain(int windowNumber);
        [DllImport("libAvaloniaNative.dylib", EntryPoint = "avn_set_file_drag_event")]
        internal static extern int Set(nint eventHandle);
        [DllImport("libAvaloniaNative.dylib", EntryPoint = "avn_release_file_drag_event")]
        internal static extern void Release(nint eventHandle);
    }
}

[System.Text.Json.Serialization.JsonSerializable(typeof(string[]))]
internal partial class FileDragJsonContext : System.Text.Json.Serialization.JsonSerializerContext;
