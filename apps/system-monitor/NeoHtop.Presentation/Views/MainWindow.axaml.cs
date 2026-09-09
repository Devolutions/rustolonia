using System;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using Avalonia.Rust.Interop;
using NeoHtop.Presentation.Generated;

namespace NeoHtop.Presentation.Views;

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
        Closed += (_, _) =>
        {
            _menu?.Dispose();
            _adapter.Dispose();
        };
    }

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);

    private void OnAppInfoClick(object? sender, RoutedEventArgs e)
    {
        if (this.FindControl<Border>("AppInfoOverlay") is { } overlay)
            overlay.IsVisible = !overlay.IsVisible;
    }
}
