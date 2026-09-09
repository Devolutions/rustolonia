using System;
using Avalonia.Controls;
using Avalonia.Host.Ownership.MicroCom;
using MicroCom.Runtime;

namespace Avalonia.Host.Ownership;

internal sealed class MicroComOwnershipProbe :
    HostNativeOwned,
    IAvnMicroComOwnershipProbe
{
    private Button? _button = new();

    public MicroComOwnershipProbe(Action<Action>? scheduleCleanup = null)
        : base(scheduleCleanup)
    {
        _button.Click += OnClick;
        ProjectionDiagnostics.SubscriptionAdded();
    }

    public unsafe void GetValue(int* value)
    {
        using var call = EnterCall();
        if (value is null)
            throw new ArgumentNullException(nameof(value));
        OnGetValue?.Invoke();
        *value = _button is null ? 0 : 42;
    }

    internal Action? OnGetValue { get; set; }

    public nint GetNativePointer() =>
        MicroComRuntime.GetNativeIntPtr<IAvnMicroComOwnershipProbe>(
            this,
            true);

    protected override void Destroyed()
    {
        if (_button is null)
            return;
        _button.Click -= OnClick;
        _button = null;
        ProjectionDiagnostics.SubscriptionRemoved();
    }

    private static void OnClick(object? sender, Interactivity.RoutedEventArgs e)
    {
    }
}
