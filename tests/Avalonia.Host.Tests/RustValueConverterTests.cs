using System;
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Data;
using Avalonia.Data.Converters;
using Avalonia.Rust;
using Avalonia.Rust.Interop;
using Xunit;

namespace Avalonia.Host.Tests;

public class RustValueConverterTests : IDisposable
{
    public RustValueConverterTests() => RustValueConverterRuntime.Register(null);

    public void Dispose() => RustValueConverterRuntime.Register(null);

    [Fact]
    public void Convert_dispatches_tagged_value_through_provider()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        var result = converter.Convert(42L, typeof(string), null, CultureInfo.InvariantCulture);

        Assert.Equal("Rust count: 42", result);
        Assert.Equal(1, provider.Calls);
        Assert.Equal(AvnValueKind.Int64, provider.LastValueKind);
        Assert.Equal(42, provider.LastValueInt64);
    }

    [Fact]
    public void Convert_passes_through_null_and_unset_and_do_nothing()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        Assert.Null(converter.Convert(null, typeof(string), null, CultureInfo.InvariantCulture));
        Assert.Same(
            AvaloniaProperty.UnsetValue,
            converter.Convert(AvaloniaProperty.UnsetValue, typeof(string), null, CultureInfo.InvariantCulture));
        Assert.Same(
            BindingOperations.DoNothing,
            converter.Convert(BindingOperations.DoNothing, typeof(string), null, CultureInfo.InvariantCulture));
    }

    [Fact]
    public void ConvertBack_returns_do_nothing_when_converter_does_not_support_it()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        var result = converter.ConvertBack("Rust count: 42", typeof(long), null, CultureInfo.InvariantCulture);

        Assert.Same(BindingOperations.DoNothing, result);
        Assert.Equal(0, provider.Calls);
    }

    [Fact]
    public void ConvertBack_dispatches_when_converter_supports_it()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: true);

        var result = converter.ConvertBack("count=7", typeof(long), null, CultureInfo.InvariantCulture);

        Assert.Equal(7L, result);
        Assert.Equal(RustConversionDirection.ConvertBack, provider.LastDirection);
    }

    [Fact]
    public void Convert_maps_provider_failure_to_binding_notification_with_message()
    {
        var provider = new FakeValueConverterProvider { FailWithMessage = "boom" };
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        var result = converter.Convert(1L, typeof(string), null, CultureInfo.InvariantCulture);

        var notification = Assert.IsType<BindingNotification>(result);
        Assert.Equal(BindingErrorType.Error, notification.ErrorType);
        Assert.Contains("boom", notification.Error!.Message);
    }

    [Fact]
    public void Convert_rejects_unsupported_managed_values()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        var result = converter.Convert(new object(), typeof(string), null, CultureInfo.InvariantCulture);

        var notification = Assert.IsType<BindingNotification>(result);
        Assert.Equal(BindingErrorType.Error, notification.ErrorType);
        Assert.IsType<NotSupportedException>(notification.Error);
        Assert.Equal(0, provider.Calls);
    }

    [Fact]
    public void Convert_without_registered_provider_returns_error_notification()
    {
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        var result = converter.Convert(1L, typeof(string), null, CultureInfo.InvariantCulture);

        var notification = Assert.IsType<BindingNotification>(result);
        Assert.Equal(BindingErrorType.Error, notification.ErrorType);
    }

    [Fact]
    public void Registering_the_same_provider_instance_twice_succeeds()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        RustValueConverterRuntime.Register(provider);

        Assert.Same(provider, RustValueConverterRuntime.Provider);
    }

    [Fact]
    public void Registering_a_conflicting_provider_throws()
    {
        RustValueConverterRuntime.Register(new FakeValueConverterProvider());

        Assert.Throws<InvalidOperationException>(
            () => RustValueConverterRuntime.Register(new FakeValueConverterProvider()));
    }

    [Fact]
    public void Unregister_only_clears_when_still_the_active_provider()
    {
        var first = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(first);

        RustValueConverterRuntime.Unregister(new FakeValueConverterProvider());
        Assert.Same(first, RustValueConverterRuntime.Provider);

        RustValueConverterRuntime.Unregister(first);
        Assert.Null(RustValueConverterRuntime.Provider);
    }

    [Fact]
    public void Registering_null_clears_the_provider()
    {
        RustValueConverterRuntime.Register(new FakeValueConverterProvider());
        RustValueConverterRuntime.Register(null);

        Assert.Null(RustValueConverterRuntime.Provider);
    }

    [Fact]
    public void Sample_count_to_label_converter_updates_synchronously_from_a_rust_command()
    {
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);

        // Use the reflectable adapter with an explicit synchronous dispatch
        // function so this test exercises the converter/registry contract
        // without depending on Avalonia.Threading.Dispatcher.UIThread's
        // process-wide thread affinity, which is unrelated to what this test
        // verifies and would otherwise make the test order-sensitive.
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            Avalonia.Rust.Sample.Generated.SampleViewModelMetadata.Descriptor,
            dispatch: action => action());
        var typeInfo = ((IReflectableType)adapter).GetTypeInfo();
        var count = typeInfo.GetProperty("Count")!;
        var increment = (System.Windows.Input.ICommand)typeInfo.GetProperty("IncrementCommand")!.GetValue(adapter)!;
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        Assert.Equal(
            "Rust count: 2",
            converter.Convert(count.GetValue(adapter), typeof(string), null, CultureInfo.InvariantCulture));

        // A synchronous Rust command (Increment) updates the bound property;
        // the converter must re-resolve the freshly updated value without
        // deadlocking the dispatcher/provider registry.
        increment.Execute(null);

        Assert.Equal(
            "Rust count: 3",
            converter.Convert(count.GetValue(adapter), typeof(string), null, CultureInfo.InvariantCulture));
    }

    [Fact]
    public void Converter_resolves_for_a_data_template_realized_after_registration()
    {
        // Register the provider first (as AppScope::register_value_converters
        // does at application startup), then realize the template later —
        // mirroring a ListBox/ContentPresenter that defers building its
        // DataTemplate until layout, well after window construction.
        var provider = new FakeValueConverterProvider();
        RustValueConverterRuntime.Register(provider);
        var converter = new RustValueConverter(1, supportsConvertBack: false);

        IDataTemplate template = new FuncDataTemplate<long>((value, _) => new TextBlock
        {
            Text = (string?)converter.Convert(value, typeof(string), null, CultureInfo.InvariantCulture),
        });

        var built = Assert.IsType<TextBlock>(template.Build(99L));

        Assert.Equal("Rust count: 99", built.Text);
    }

    private sealed class Model : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;
        private long _count = 2;

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = sink;
            sink!.SetString(1, "Initial");
            sink.SetInteger(2, _count);
            return 0;
        }

        public int Detach()
        {
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value) => 0;
        public int SetInteger(int propertyId, long value) => unchecked((int)0x80070057);
        public int SetBoolean(int propertyId, int value) => unchecked((int)0x80070057);
        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);

        public int Execute(int commandId, string? parameter)
        {
            if (commandId != 1)
                return unchecked((int)0x80070057);
            _count++;
            _sink!.SetInteger(2, _count);
            return 0;
        }

        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class FakeValueConverterProvider : IAvnRustValueConverterProvider
    {
        public int Calls { get; private set; }
        public AvnValueKind LastValueKind { get; private set; }
        public long LastValueInt64 { get; private set; }
        public RustConversionDirection LastDirection { get; private set; }
        public string? FailWithMessage { get; set; }

        public int Convert(
            int converterId,
            int direction,
            int valueKind,
            long valueInt64,
            double valueDouble,
            int valueBoolean,
            string? valueString,
            int parameterKind,
            long parameterInt64,
            double parameterDouble,
            int parameterBoolean,
            string? parameterString,
            int targetKind,
            string? culture,
            out int resultKind,
            out long resultInt64,
            out double resultDouble,
            out int resultBoolean,
            out nint resultString,
            out nint error)
        {
            Calls++;
            LastValueKind = (AvnValueKind)valueKind;
            LastValueInt64 = valueInt64;
            LastDirection = (RustConversionDirection)direction;

            resultKind = (int)AvnValueKind.Null;
            resultInt64 = 0;
            resultDouble = 0;
            resultBoolean = 0;
            resultString = 0;
            error = 0;

            if (FailWithMessage is { } message)
            {
                error = Marshal.StringToCoTaskMemUni(message);
                return unchecked((int)0x80004005); // E_FAIL
            }

            if (converterId != 1)
            {
                error = Marshal.StringToCoTaskMemUni($"unknown converter {converterId}");
                return unchecked((int)0x80070057); // E_INVALIDARG
            }

            if ((AvnValueKind)valueKind == AvnValueKind.UnsetValue)
            {
                resultKind = (int)AvnValueKind.UnsetValue;
                return 0;
            }

            if ((AvnValueKind)valueKind == AvnValueKind.Null)
            {
                resultKind = (int)AvnValueKind.Null;
                return 0;
            }

            if ((AvnValueKind)valueKind == AvnValueKind.DoNothing)
            {
                resultKind = (int)AvnValueKind.DoNothing;
                return 0;
            }

            if ((RustConversionDirection)direction == RustConversionDirection.ConvertBack)
            {
                if ((AvnValueKind)valueKind != AvnValueKind.String ||
                    valueString is null ||
                    !valueString.StartsWith("count=", StringComparison.Ordinal) ||
                    !long.TryParse(valueString.AsSpan(6), out var parsed))
                {
                    error = Marshal.StringToCoTaskMemUni("expected 'count=<n>'");
                    return unchecked((int)0x80070057);
                }

                resultKind = (int)AvnValueKind.Int64;
                resultInt64 = parsed;
                return 0;
            }

            if ((AvnValueKind)valueKind != AvnValueKind.Int64)
            {
                error = Marshal.StringToCoTaskMemUni("expected an Int64 value");
                return unchecked((int)0x80070057);
            }

            resultKind = (int)AvnValueKind.String;
            resultString = Marshal.StringToCoTaskMemUni($"Rust count: {valueInt64}");
            return 0;
        }
    }
}
