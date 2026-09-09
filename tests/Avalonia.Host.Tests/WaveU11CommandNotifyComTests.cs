using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveU11CommandNotifyComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Foreign_command_adapter_subscribes_and_raises_can_execute_changed()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        var foreign = new ForeignCommand();
        var button = Target<Button>(projected);
        button.Command = AvnCommand.ToCommand(foreign);

        var raised = 0;
        void OnCanExecuteChanged(object? sender, EventArgs e) => raised++;
        button.Command!.CanExecuteChanged += OnCanExecuteChanged;

        Assert.Equal(1, foreign.AdvisedCount);
        // The adapter subscribed eagerly when ToCommand wrapped the command.

        foreign.RaiseCanExecuteChanged();
        Assert.Equal(1, raised);

        button.Command!.CanExecuteChanged -= OnCanExecuteChanged;
        foreign.UnadviseAll();

        button.Command = null;
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));

    /// A command implemented outside the host assembly, standing in for the
    /// Rust CCW: advise stores the handler, invoke forwards to it.
    private sealed class ForeignCommand : IAvnCommand
    {
        private readonly System.Collections.Generic.Dictionary<long, IAvnCommandCanExecuteChangedHandler> _handlers = new();
        private long _nextId = 1;

        public int AdvisedCount => _handlers.Count;

        public int Execute(AvnVariant parameter) => 0;

        public int CanExecute(AvnVariant parameter, out int value)
        {
            value = 1;
            return 0;
        }

        public int AdviseCanExecuteChanged(IAvnCommandCanExecuteChangedHandler? handler, out long subscriptionId)
        {
            subscriptionId = 0;
            if (handler is null)
                return unchecked((int)0x80004003);
            var id = _nextId++;
            _handlers[id] = handler;
            subscriptionId = id;
            return 0;
        }

        public int UnadviseCanExecuteChanged(long subscriptionId) =>
            _handlers.Remove(subscriptionId) ? 0 : unchecked((int)0x80004005);

        public void RaiseCanExecuteChanged()
        {
            foreach (var handler in _handlers.Values)
                Assert.Equal(0, handler.Invoke());
        }

        public void UnadviseAll() => _handlers.Clear();
    }
}
