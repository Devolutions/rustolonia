using System;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Threading;
using Avalonia.Data;
using Avalonia.Data.Converters;
using Avalonia.Rust.Interop;

namespace Avalonia.Rust;

/// <summary>
/// Tag for the scalar values carried across the Rust value-converter ABI.
/// Kept in sync with <c>ScalarKind</c> in <c>avalonia-sys::value_converter</c>.
/// </summary>
public enum AvnValueKind
{
    Null = 0,
    String = 1,
    Int64 = 2,
    Boolean = 3,
    Double = 4,
    UnsetValue = 5,
    DoNothing = 6,

    /// <summary>
    /// Used only as a target-type hint when the binding target type does not
    /// map to a supported scalar kind. Never a legal value/parameter/result
    /// kind.
    /// </summary>
    Any = 7,
}

/// <summary>
/// Direction of a Rust value-converter call.
/// </summary>
public enum RustConversionDirection
{
    Convert = 0,
    ConvertBack = 1,
}

/// <summary>
/// Process-wide, lock-free registry for the single Rust value-converter
/// provider registered for the lifetime of the running application. Reading
/// <see cref="Provider"/> is safe from any thread and at any time, including
/// long after window construction, so converters resolve correctly for
/// deferred DataTemplate/ControlTemplate realization.
/// </summary>
public static class RustValueConverterRuntime
{
    private static IAvnRustValueConverterProvider? _provider;

    /// <summary>
    /// The currently registered provider, or null when no Rust application
    /// has registered value converters. Applications and views that never
    /// use Rust value converters are unaffected.
    /// </summary>
    public static IAvnRustValueConverterProvider? Provider => Volatile.Read(ref _provider);

    /// <summary>
    /// Registers <paramref name="provider"/> as the application-scoped
    /// converter provider, or clears the registration when it is null.
    /// </summary>
    /// <exception cref="InvalidOperationException">
    /// A different provider is already registered. Registrations are
    /// rejected rather than silently replaced so a realized DataTemplate
    /// cannot have its converters swapped out from under it.
    /// </exception>
    public static void Register(IAvnRustValueConverterProvider? provider)
    {
        if (provider is null)
        {
            Interlocked.Exchange(ref _provider, null);
            return;
        }

        var existing = Interlocked.CompareExchange(ref _provider, provider, null);
        if (existing is not null && !ReferenceEquals(existing, provider))
        {
            throw new InvalidOperationException(
                "A Rust value converter provider is already registered for this application. " +
                "Only one application-scoped provider may be active at a time.");
        }
    }

    /// <summary>
    /// Clears the registration if, and only if, <paramref name="provider"/>
    /// is still the currently registered instance.
    /// </summary>
    public static void Unregister(IAvnRustValueConverterProvider provider) =>
        Interlocked.CompareExchange(ref _provider, null, provider);
}

/// <summary>
/// An <see cref="IValueConverter"/> that dispatches to a named Rust converter
/// resolved through <see cref="RustValueConverterRuntime"/>. Instances are
/// created by generated code (one per converter declared in the view-model
/// IR); application authors consume the generated static instances rather
/// than this type or raw converter IDs directly.
/// </summary>
/// <remarks>
/// Only scalar values are supported: null, string, integral/floating-point
/// numbers, Boolean, <see cref="AvaloniaProperty.UnsetValue"/>, and
/// <see cref="BindingOperations.DoNothing"/>. Any other managed value is
/// rejected with a <see cref="BindingNotification"/> error rather than being
/// forwarded across the ABI.
/// </remarks>
public sealed class RustValueConverter(int converterId, bool supportsConvertBack) : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        Invoke(RustConversionDirection.Convert, value, targetType, parameter, culture);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (!supportsConvertBack)
            return BindingOperations.DoNothing;
        return Invoke(RustConversionDirection.ConvertBack, value, targetType, parameter, culture);
    }

    private object? Invoke(
        RustConversionDirection direction,
        object? value,
        Type targetType,
        object? parameter,
        CultureInfo culture)
    {
        var provider = RustValueConverterRuntime.Provider;
        if (provider is null)
        {
            return Error(
                $"No Rust value converter provider is registered; converter {converterId} cannot resolve.");
        }

        try
        {
            ToScalar(value, out var valueKind, out var valueInt64, out var valueDouble, out var valueBoolean, out var valueString);
            ToScalar(parameter, out var parameterKind, out var parameterInt64, out var parameterDouble, out var parameterBoolean, out var parameterString);
            var targetKind = ToTargetKind(targetType);

            var hresult = provider.Convert(
                converterId,
                (int)direction,
                (int)valueKind, valueInt64, valueDouble, valueBoolean, valueString,
                (int)parameterKind, parameterInt64, parameterDouble, parameterBoolean, parameterString,
                (int)targetKind,
                culture.Name,
                out var resultKind,
                out var resultInt64,
                out var resultDouble,
                out var resultBoolean,
                out var resultStringPtr,
                out var errorPtr);

            var errorMessage = TakeString(errorPtr);
            if (hresult < 0)
            {
                return Error(errorMessage
                    ?? $"Rust value converter {converterId} failed with HRESULT 0x{hresult:X8}.");
            }

            return FromScalar((AvnValueKind)resultKind, resultInt64, resultDouble, resultBoolean, TakeString(resultStringPtr));
        }
        catch (Exception e) when (e is not NotSupportedException)
        {
            return Error(e);
        }
        catch (NotSupportedException e)
        {
            return Error(e);
        }
    }

    private static BindingNotification Error(string message) =>
        new(new InvalidOperationException(message), BindingErrorType.Error);

    private static BindingNotification Error(Exception exception) =>
        new(exception, BindingErrorType.Error);

    private static string? TakeString(nint ptr)
    {
        if (ptr == 0)
            return null;
        var value = Marshal.PtrToStringUni(ptr);
        Marshal.FreeCoTaskMem(ptr);
        return value;
    }

    private static void ToScalar(
        object? value,
        out AvnValueKind kind,
        out long int64Value,
        out double doubleValue,
        out int booleanValue,
        out string? stringValue)
    {
        int64Value = 0;
        doubleValue = 0;
        booleanValue = 0;
        stringValue = null;

        switch (value)
        {
            case null:
                kind = AvnValueKind.Null;
                return;
            case string s:
                kind = AvnValueKind.String;
                stringValue = s;
                return;
            case bool b:
                kind = AvnValueKind.Boolean;
                booleanValue = b ? 1 : 0;
                return;
            case sbyte or byte or short or ushort or int or uint or long or ulong:
                kind = AvnValueKind.Int64;
                int64Value = System.Convert.ToInt64(value, CultureInfo.InvariantCulture);
                return;
            case float or double:
                kind = AvnValueKind.Double;
                doubleValue = System.Convert.ToDouble(value, CultureInfo.InvariantCulture);
                return;
        }

        if (ReferenceEquals(value, AvaloniaProperty.UnsetValue))
        {
            kind = AvnValueKind.UnsetValue;
            return;
        }

        if (ReferenceEquals(value, BindingOperations.DoNothing))
        {
            kind = AvnValueKind.DoNothing;
            return;
        }

        throw new NotSupportedException(
            "Rust value converters only support scalar values (null, string, integer, " +
            $"boolean, double, UnsetValue, DoNothing); received '{value.GetType()}'.");
    }

    private static object? FromScalar(
        AvnValueKind kind,
        long int64Value,
        double doubleValue,
        int booleanValue,
        string? stringValue) => kind switch
        {
            AvnValueKind.Null => null,
            AvnValueKind.String => stringValue,
            AvnValueKind.Int64 => int64Value,
            AvnValueKind.Boolean => booleanValue != 0,
            AvnValueKind.Double => doubleValue,
            AvnValueKind.UnsetValue => AvaloniaProperty.UnsetValue,
            AvnValueKind.DoNothing => BindingOperations.DoNothing,
            _ => throw new NotSupportedException(
                $"Rust value converter returned an unsupported value kind '{kind}'."),
        };

    private static AvnValueKind ToTargetKind(Type targetType)
    {
        if (targetType == typeof(string))
            return AvnValueKind.String;
        if (targetType == typeof(bool) || targetType == typeof(bool?))
            return AvnValueKind.Boolean;
        if (targetType == typeof(double) || targetType == typeof(double?) ||
            targetType == typeof(float) || targetType == typeof(float?))
            return AvnValueKind.Double;
        if (targetType == typeof(sbyte) || targetType == typeof(byte) ||
            targetType == typeof(short) || targetType == typeof(ushort) ||
            targetType == typeof(int) || targetType == typeof(uint) ||
            targetType == typeof(long) || targetType == typeof(ulong) ||
            targetType == typeof(sbyte?) || targetType == typeof(byte?) ||
            targetType == typeof(short?) || targetType == typeof(ushort?) ||
            targetType == typeof(int?) || targetType == typeof(uint?) ||
            targetType == typeof(long?) || targetType == typeof(ulong?))
            return AvnValueKind.Int64;
        return AvnValueKind.Any;
    }
}
