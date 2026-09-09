using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using Avalonia.Rust;
using Avalonia.Rust.Sample.Generated;

namespace Avalonia.Rust.Sample.Views;

public partial class RustDynamicVmWindow : Window
{
    public RustDynamicVmWindow()
    {
        InitializeComponent();
    }

    private void InitializeComponent() =>
        AvaloniaXamlLoader.Load(this);

    /// <summary>
    /// Cycles the `Priority` enum property through its members via the
    /// dynamic reflectable adapter, exactly as a real data-bound control
    /// would through <see cref="RustBindingExtension"/>: reading and writing
    /// the concrete generated enum type, not a raw integer.
    /// </summary>
    private void OnCyclePriorityClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not ReflectableRustViewModelAdapter adapter)
            return;

        var current = (Priority)adapter.GetMemberValue("Priority")!;
        var next = current switch
        {
            Priority.Low => Priority.Normal,
            Priority.Normal => Priority.High,
            _ => Priority.Low,
        };
        adapter.SetMemberValue("Priority", next);
    }
}

