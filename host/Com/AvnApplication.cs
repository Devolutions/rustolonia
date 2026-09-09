using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Generated.ViewModels;
using Avalonia.Input;
using Avalonia.Rust.Interop;
using Avalonia.Styling;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

[GeneratedComClass]
public partial class AvnApplication : IAvnApplication, IAvnApplication2
{
    private ClassicDesktopStyleApplicationLifetime? _lifetime;
    private readonly AvnAsyncOperations _asyncOperations = new();

    public int Run(IAvnAppHandler? handler)
    {
        if (handler is null)
            return HResults.E_POINTER;

        try
        {
            using var platformThread = RustHostPlatform.EnterThread();

            // Startup/open-with arguments are supplied by the Rust entry point
            // through IAvnApplication3 before Run, because a Rust executable
            // owns argv and the managed host never sees it.
            var startupArguments = SnapshotStartupArguments();
            var lifetime = new ClassicDesktopStyleApplicationLifetime
            {
                ShutdownMode = ShutdownMode.OnLastWindowClose,
                Args = startupArguments,
            };
            _lifetime = lifetime;

            RustHostPlatform.Configure(AppBuilder.Configure<HostApplication>())
                .SetupWithLifetime(lifetime);

            lifetime.Startup += (_, _) =>
            {
                var started = handler.OnStarted();
                if (started < 0)
                    Marshal.ThrowExceptionForHR(started);
            };

            lifetime.Start(startupArguments);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
        finally
        {
            ReleaseDesktopFileIntegration();
        }
    }

    public int Shutdown()
    {
        try
        {
            _asyncOperations.CancelAll();
            _lifetime?.Shutdown();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartDelay(
            int milliseconds,
            IAvnAsyncCompletion? completion,
            out long operationId)
    {
        operationId = 0;
        if (milliseconds < 0)
            return HResults.E_INVALIDARG;
        return _asyncOperations.Start(
            completion,
            async cancellation =>
            {
                await Task.Delay(milliseconds, cancellation);
                return AvnAsyncValue.None;
            },
            out operationId);
    }

    public int StartClipboardSetText(
            IAvnWindow? window,
            string? text,
            IAvnAsyncCompletion? completion,
            out long operationId)
    {
        operationId = 0;
        if (window is null || text is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var managedWindow = (Window?)ProjectionRuntime.Unwrap(window)
                ?? throw new ObjectDisposedException(nameof(window));
            var clipboard = managedWindow.Clipboard
                ?? throw new NotSupportedException("The top-level has no clipboard.");
            return _asyncOperations.Start(
                completion,
                async cancellation =>
                {
                    cancellation.ThrowIfCancellationRequested();
                    var data = new DataTransfer();
                    data.Add(DataTransferItem.CreateText(text));
                    await clipboard.SetDataAsync(data);
                    cancellation.ThrowIfCancellationRequested();
                    return AvnAsyncValue.None;
                },
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int StartClipboardGetText(
            IAvnWindow? window,
            IAvnAsyncCompletion? completion,
            out long operationId)
    {
        operationId = 0;
        if (window is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var managedWindow = (Window?)ProjectionRuntime.Unwrap(window)
                ?? throw new ObjectDisposedException(nameof(window));
            var clipboard = managedWindow.Clipboard
                ?? throw new NotSupportedException("The top-level has no clipboard.");
            return _asyncOperations.Start(
                completion,
                async cancellation =>
                {
                    cancellation.ThrowIfCancellationRequested();
                    using var data = await clipboard.TryGetDataAsync();
                    var text = data is null ? null : await data.TryGetTextAsync();
                    cancellation.ThrowIfCancellationRequested();
                    return AvnAsyncValue.FromString(text);
                },
                out operationId);
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int CancelAsyncOperation(long operationId) =>
        _asyncOperations.Cancel(operationId);

    public int CreateRustVmWindow(
        int viewId,
        IAvnRustViewModel? model,
        out IAvnWindow? window)
    {
        window = null;
        if (model is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var managedWindow = RustViewRegistry.Create(viewId, model);
            window = (IAvnWindow)ProjectionRuntime.Wrap(managedWindow)!;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int SetValueConverterProvider(IAvnRustValueConverterProvider? provider)
    {
        try
        {
            Rust.RustValueConverterRuntime.Register(provider);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int GetRequestedThemeVariant(out int value)
    {
        value = 0;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            value = ToAbiTheme(Application.Current?.RequestedThemeVariant);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int SetRequestedThemeVariant(int value)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var application = Application.Current
                ?? throw new InvalidOperationException("Avalonia application is not running.");
            application.RequestedThemeVariant = FromAbiTheme(value);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int GetActualThemeVariant(out int value)
    {
        value = 0;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            value = ToAbiTheme(Application.Current?.ActualThemeVariant);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int TryGetResource(
        string? key,
        int themeVariant,
        out int found,
        out IAvnResourceValue? value)
    {
        found = 0;
        value = null;
        if (key is null)
            return HResults.E_POINTER;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            var application = Application.Current
                ?? throw new InvalidOperationException("Avalonia application is not running.");
            if (!application.TryGetResource(key, FromAbiTheme(themeVariant), out var resource))
                return HResults.S_OK;
            value = new AvnResourceValue(resource);
            found = 1;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    private static ThemeVariant FromAbiTheme(int value) => value switch
    {
        0 => ThemeVariant.Default,
        1 => ThemeVariant.Light,
        2 => ThemeVariant.Dark,
        _ => throw new ArgumentOutOfRangeException(nameof(value)),
    };

    private static int ToAbiTheme(ThemeVariant? value)
    {
        if (value is null || value == ThemeVariant.Default)
            return 0;
        if (value == ThemeVariant.Light)
            return 1;
        if (value == ThemeVariant.Dark)
            return 2;
        throw new NotSupportedException($"Custom theme variant '{value.Key}' is not supported by the ABI.");
    }

}
