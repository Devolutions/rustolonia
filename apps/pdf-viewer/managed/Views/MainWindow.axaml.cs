using System;
using System.ComponentModel;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Markup.Xaml;
using Avalonia.Rust.Interop;
using Avalonia.Threading;
using PdfViewer.Presentation.Generated;

namespace PdfViewer.Presentation.Views;

public partial class MainWindow : Window
{
    private readonly MainViewModelAdapter _adapter;
    private IDisposable? _menu;

    public MainWindow()
    {
        InitializeComponent();
        _adapter = null!;
    }

    public MainWindow(IAvnRustViewModel model)
        : this()
    {
        _adapter = new MainViewModelAdapter(model);
        DataContext = _adapter;
        _menu = MainViewModelMenus.AttachMain(this, _adapter);
        _adapter.PropertyChanged += OnAdapterPropertyChanged;
        Closed += (_, _) =>
        {
            _adapter.PropertyChanged -= OnAdapterPropertyChanged;
            _menu?.Dispose();
            _adapter.Dispose();
        };
    }

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);

    private void OnAdapterPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(MainViewModelAdapter.PageImage))
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (this.FindControl<ScrollViewer>("PageScrollViewer") is { } viewer)
                    viewer.Offset = Vector.Zero;
            });
        }
    }

    private void OnPageWheelChanged(object? sender, PointerWheelEventArgs e)
    {
        if (_adapter.IsLoading)
            return;

        if (this.FindControl<ScrollViewer>("PageScrollViewer") is not { } viewer)
            return;

        var offset = viewer.Offset.Y;
        var maximum = Math.Max(0, viewer.Extent.Height - viewer.Viewport.Height);
        var atTop = offset <= 0.5;
        var atBottom = offset >= maximum - 0.5;

        if (e.Delta.Y > 0 && atTop && _adapter.CanGoPrevious)
        {
            _adapter.PreviousPageCommand.Execute(null);
            e.Handled = true;
        }
        else if (e.Delta.Y < 0 && atBottom && _adapter.CanGoNext)
        {
            _adapter.NextPageCommand.Execute(null);
            e.Handled = true;
        }
    }
}
