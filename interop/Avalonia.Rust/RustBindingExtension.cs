using System;
using System.ComponentModel;
using Avalonia.Data;
using Avalonia.Data.Core;
using Avalonia.Data.Core.Plugins;

namespace Avalonia.Rust;

public sealed class RustBindingExtension : CompiledBinding
{
    public RustBindingExtension(string path)
        : base(CreatePath(path))
    {
    }

    public CompiledBinding ProvideValue(IServiceProvider serviceProvider) => this;

    private static CompiledBindingPath CreatePath(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        var property = new ClrPropertyInfo(
            path,
            target => GetAdapter(target).GetMemberValue(path),
            (target, value) => GetAdapter(target).SetMemberValue(path, value),
            typeof(object));
        return new CompiledBindingPathBuilder()
            .Property(property, static (target, info) => new RustPropertyAccessor(target, info))
            .Build();
    }

    private static ReflectableRustViewModelAdapter GetAdapter(object target) =>
        target as ReflectableRustViewModelAdapter ??
        throw new InvalidOperationException(
            $"RustBinding requires a {nameof(ReflectableRustViewModelAdapter)} data context.");

    private sealed class RustPropertyAccessor(
        WeakReference<object?> target,
        IPropertyInfo property) : IPropertyAccessor
    {
        private Action<object?>? _listener;
        private WeakPropertyChangedSubscription? _subscription;
        private int _eventVersion;

        public Type PropertyType => property.PropertyType;

        public object? Value =>
            target.TryGetTarget(out var value) && value is not null
                ? property.Get(value)
                : null;

        public bool SetValue(object? value, BindingPriority priority)
        {
            if (!property.CanSet || !target.TryGetTarget(out var instance) || instance is null)
                return false;

            var eventVersion = _eventVersion;
            property.Set(instance, value);
            if (_eventVersion == eventVersion)
                PublishValue();
            return true;
        }

        public void Subscribe(Action<object?> listener)
        {
            ArgumentNullException.ThrowIfNull(listener);
            Unsubscribe();
            _listener = listener;
            if (target.TryGetTarget(out var instance) &&
                instance is INotifyPropertyChanged notifier)
            {
                _subscription = new WeakPropertyChangedSubscription(this, notifier);
            }
            PublishValue();
        }

        public void Unsubscribe()
        {
            _subscription?.Dispose();
            _subscription = null;
            _listener = null;
        }

        public void Dispose() => Unsubscribe();

        private void OnPropertyChanged(PropertyChangedEventArgs args)
        {
            if (string.IsNullOrEmpty(args.PropertyName) || args.PropertyName == property.Name)
            {
                _eventVersion++;
                PublishValue();
            }
        }

        private void PublishValue()
        {
            try
            {
                _listener?.Invoke(Value);
            }
            catch (Exception exception)
            {
                _listener?.Invoke(new BindingNotification(exception, BindingErrorType.Error));
            }
        }

        private sealed class WeakPropertyChangedSubscription : IDisposable
        {
            private readonly WeakReference<RustPropertyAccessor> _owner;
            private readonly WeakReference<INotifyPropertyChanged> _source;

            public WeakPropertyChangedSubscription(
                RustPropertyAccessor owner,
                INotifyPropertyChanged source)
            {
                _owner = new WeakReference<RustPropertyAccessor>(owner);
                _source = new WeakReference<INotifyPropertyChanged>(source);
                source.PropertyChanged += OnPropertyChanged;
            }

            public void Dispose()
            {
                if (_source.TryGetTarget(out var source))
                    source.PropertyChanged -= OnPropertyChanged;
            }

            private void OnPropertyChanged(object? sender, PropertyChangedEventArgs args)
            {
                if (_owner.TryGetTarget(out var owner))
                {
                    owner.OnPropertyChanged(args);
                }
                else if (sender is INotifyPropertyChanged source)
                {
                    source.PropertyChanged -= OnPropertyChanged;
                }
            }
        }
    }
}
