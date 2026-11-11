using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Media;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class StyleSheetAndKeyBindingTests
{
    [Fact]
    public void Scoped_style_sheet_applies_and_removes_setters_and_transitions()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var hostApp = new AvnApplication();
        var factory = new AvnControlFactory();

        Assert.Equal(0, factory.CreateStackPanel(out var panelCom));
        Assert.Equal(0, factory.CreateButton(out var buttonCom));
        Assert.NotNull(panelCom);
        Assert.NotNull(buttonCom);

        var panel = Assert.IsType<StackPanel>(ProjectionRuntime.Unwrap((IAvnControl)panelCom));
        var button = Assert.IsType<Button>(ProjectionRuntime.Unwrap((IAvnControl)buttonCom));
        button.Classes.Add("primary");
        panel.Children.Add(button);
        var root = new TestRoot { Child = panel };

        const string json = """
        {
          "rules": [
            {
              "target": "Button",
              "classes": ["primary"],
              "setters": [
                { "prop": "background", "value": { "kind": "solid", "argb": 4280640491, "opacity": 1.0 } },
                { "prop": "cornerRadius", "value": [8, 8, 8, 8] }
              ],
              "transitions": [
                { "prop": "opacity", "durationMs": 120 }
              ]
            }
          ]
        }
        """;

        Assert.Equal(0, hostApp.ApplyStyles((IAvnControl)panelCom, json, out var styleHandle));
        Assert.True(styleHandle > 0);
        root.Measure(Size.Infinity);
        Assert.Equal(new CornerRadius(8), button.CornerRadius);
        var solid = Assert.IsAssignableFrom<ISolidColorBrush>(button.Background);
        Assert.Equal(Color.FromUInt32(4280640491), solid.Color);

        Assert.Equal(0, hostApp.RemoveStyles(styleHandle));
        root.Measure(Size.Infinity);
        Assert.Equal(new CornerRadius(0), button.CornerRadius);
    }

    [Fact]
    public void Window_key_bindings_and_collection_insert_work_in_place()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var hostApp = new AvnApplication();
        var factory = new AvnControlFactory();

        Assert.Equal(0, factory.CreateWindow(out var winCom));
        Assert.NotNull(winCom);
        var window = Assert.IsType<Window>(ProjectionRuntime.Unwrap((IAvnControl)winCom));

        var command = AvnCommand.FromCommand(new StubCommand());
        Assert.Equal(0, hostApp.AddWindowKeyBinding(winCom, "Ctrl+S", command, default, out var bindingId));
        Assert.Single(window.KeyBindings);
        Assert.Equal(0, hostApp.RemoveWindowKeyBinding(winCom, bindingId));
        Assert.Empty(window.KeyBindings);

        Assert.Equal(0, factory.CreateStackPanel(out var stackCom));
        Assert.Equal(0, factory.CreateTextBlock(out var firstTb));
        Assert.Equal(0, factory.CreateTextBlock(out var middleTb));
        Assert.Equal(0, factory.CreateTextBlock(out var lastTb));
        Assert.Equal(0, stackCom!.GetChildren(out var children));
        Assert.Equal(0, children!.Add((IAvnControl)firstTb!));
        Assert.Equal(0, children.Add((IAvnControl)lastTb!));
        Assert.Equal(0, children.Insert(1, (IAvnControl)middleTb!));
        Assert.Equal(0, children.GetCount(out var count));
        Assert.Equal(3, count);
        Assert.Equal(0, children.GetAt(1, out var atOne));
        Assert.Same(
            ProjectionRuntime.Unwrap((IAvnControl)middleTb!),
            ProjectionRuntime.Unwrap(atOne));
    }

    private sealed class StubCommand : ICommand
    {
        public event System.EventHandler? CanExecuteChanged { add { } remove { } }
        public bool CanExecute(object? parameter) => true;
        public void Execute(object? parameter) { }
    }
}
