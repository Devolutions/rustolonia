using System.Reflection;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU7CommandComTests
{
    [Fact]
    public void Button_command_round_trips_execute()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        var executed = 0;
        Target<Button>(projected).Command = new Relay(_ => executed++);
        var command = AvnCommand.FromCommand(Target<Button>(projected).Command);
        Assert.NotNull(command);
        Assert.Equal(0, command.Execute(AvnVariant.FromObject("parameter")));
        Assert.Equal(1, executed);
        Assert.Equal(0, command.CanExecute(AvnVariant.FromObject("parameter"), out var can));
        Assert.Equal(1, can);
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));

    private sealed class Relay : ICommand
    {
        private readonly System.Action<object?> _execute;
        public Relay(System.Action<object?> execute) => _execute = execute;
        public event System.EventHandler? CanExecuteChanged { add { } remove { } }
        public bool CanExecute(object? parameter) => true;
        public void Execute(object? parameter) => _execute(parameter);
    }
}
