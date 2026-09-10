using System;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Markup.Xaml;
using Avalonia.Rust.Interop;
using ArchiveTool.Presentation.Generated;

namespace ArchiveTool.Presentation.Views;

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

    private void OnLeftActivate(object? sender, PointerPressedEventArgs e)
    {
        if (_adapter is null) return;
        if (_adapter.ActivateLeftCommand.CanExecute(null))
            _adapter.ActivateLeftCommand.Execute(null);
    }

    private void OnRightActivate(object? sender, PointerPressedEventArgs e)
    {
        if (_adapter.ActivateRightCommand.CanExecute(null))
            _adapter.ActivateRightCommand.Execute(null);
    }

    private void OnLeftGoUp(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (_adapter.ActivateLeftCommand.CanExecute(null))
            _adapter.ActivateLeftCommand.Execute(null);
        if (_adapter.GoUpCommand.CanExecute(null))
            _adapter.GoUpCommand.Execute(null);
    }

    private void OnRightGoUp(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (_adapter.ActivateRightCommand.CanExecute(null))
            _adapter.ActivateRightCommand.Execute(null);
        if (_adapter.GoUpCommand.CanExecute(null))
            _adapter.GoUpCommand.Execute(null);
    }

    private void OnLeftDoubleTapped(object? sender, TappedEventArgs e)
    {
        if (_adapter.ActivateLeftCommand.CanExecute(null))
            _adapter.ActivateLeftCommand.Execute(null);
        if (_adapter.OpenItemCommand.CanExecute(null))
            _adapter.OpenItemCommand.Execute(null);
    }

    private void OnRightDoubleTapped(object? sender, TappedEventArgs e)
    {
        if (_adapter.ActivateRightCommand.CanExecute(null))
            _adapter.ActivateRightCommand.Execute(null);
        if (_adapter.OpenItemCommand.CanExecute(null))
            _adapter.OpenItemCommand.Execute(null);
    }
}
