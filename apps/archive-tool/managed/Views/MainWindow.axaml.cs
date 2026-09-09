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

    private void OnEntriesDoubleTapped(object? sender, TappedEventArgs e)
    {
        if (_adapter.OpenItemCommand.CanExecute(null))
            _adapter.OpenItemCommand.Execute(null);
    }
}
