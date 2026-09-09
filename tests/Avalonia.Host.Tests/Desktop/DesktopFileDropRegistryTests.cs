using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Host.Desktop;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// The managed half of incoming drag-and-drop: routed-event wiring, synchronous
/// effect negotiation, and asynchronous delivery to the consumer.
/// </summary>
public class DesktopFileDropRegistryTests
{
    [Theory]
    [InlineData(false, false)]
    [InlineData(false, true)]
    [InlineData(true, false)]
    [InlineData(true, true)]
    public void Multiple_subscriptions_restore_only_after_the_last_removal(bool original, bool reverse)
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        DragDrop.SetAllowDrop(border, original);
        registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out var first);
        registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out var second);
        Assert.Equal(0, registry.Unsubscribe(reverse ? second : first));
        Assert.True(DragDrop.GetAllowDrop(border));
        Assert.Equal(0, registry.Unsubscribe(reverse ? first : second));
        Assert.Equal(original, DragDrop.GetAllowDrop(border));
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void Clear_restores_original_state_for_shared_targets(bool original)
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        DragDrop.SetAllowDrop(border, original);
        registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out _);
        registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out _);
        registry.Clear();
        registry.Clear();
        Assert.Equal(original, DragDrop.GetAllowDrop(border));
        registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out var next);
        registry.Unsubscribe(next);
        Assert.Equal(original, DragDrop.GetAllowDrop(border));
    }

    [Fact]
    public void Subscribing_enables_dropping_and_restores_the_previous_value()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        Assert.False(DragDrop.GetAllowDrop(border));

        Assert.Equal(0, registry.Subscribe(border, DragDropEffects.Copy, new RecordingHandler(), out var id));
        Assert.True(DragDrop.GetAllowDrop(border));

        Assert.Equal(0, registry.Unsubscribe(id));
        Assert.False(DragDrop.GetAllowDrop(border));
    }

    [Fact]
    public void The_platform_gets_its_effect_synchronously_and_rust_is_notified_later()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        var args = Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy | DragDropEffects.Move);

        // Synchronous: the drag loop already has its answer and Rust has not run.
        Assert.Equal(DragDropEffects.Copy, args.DragEffects);
        Assert.True(args.Handled);
        Assert.Empty(handler.Events);

        Dispatcher.UIThread.RunJobs();

        var notification = Assert.Single(handler.Events);
        Assert.Equal((int)DesktopDropEventKind.Enter, notification.Kind);
        Assert.Equal((int)(DragDropEffects.Copy | DragDropEffects.Move), notification.AllowedEffects);
        Assert.Equal((int)DragDropEffects.Copy, notification.EffectiveEffects);
        Assert.Equal(new[] { "file:///logs/a.log" }, notification.Uris);
    }

    [Fact]
    public void Enter_over_leave_and_drop_arrive_in_order()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Raise(border, DragDrop.DragOverEvent, transfer, DragDropEffects.Copy);
        Raise(border, DragDrop.DragLeaveEvent, transfer, DragDropEffects.Copy);
        Raise(border, DragDrop.DropEvent, transfer, DragDropEffects.Copy);

        Dispatcher.UIThread.RunJobs();

        Assert.Equal(
            new[]
            {
                (int)DesktopDropEventKind.Enter,
                (int)DesktopDropEventKind.Over,
                (int)DesktopDropEventKind.Leave,
                (int)DesktopDropEventKind.Drop,
            },
            handler.Events.Select(e => e.Kind));
        Assert.Empty(handler.Events[2].Uris);
    }

    [Fact]
    public void A_drag_the_subscriber_does_not_accept_is_refused()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        var args = Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Move);

        Assert.Equal(DragDropEffects.None, args.DragEffects);
        Assert.False(args.Handled);

        Dispatcher.UIThread.RunJobs();
        Assert.Equal((int)DragDropEffects.None, Assert.Single(handler.Events).EffectiveEffects);
    }

    [Fact]
    public void A_drag_without_files_is_refused_but_still_reported()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer();
        var args = Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);

        Assert.Equal(DragDropEffects.None, args.DragEffects);
        Dispatcher.UIThread.RunJobs();
        Assert.Empty(Assert.Single(handler.Events).Uris);
    }

    [Fact]
    public void A_failing_consumer_does_not_break_the_drag_loop()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new FailingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Dispatcher.UIThread.RunJobs();

        var args = Raise(border, DragDrop.DropEvent, transfer, DragDropEffects.Copy);
        Assert.Equal(DragDropEffects.Copy, args.DragEffects);
        Dispatcher.UIThread.RunJobs();

        Assert.Equal(2, handler.CallCount);
    }

    [Fact]
    public void Unsubscribing_stops_delivery_and_clearing_removes_every_subscription()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var first = new Border();
        var second = new Border();
        var firstHandler = new RecordingHandler();
        var secondHandler = new RecordingHandler();
        registry.Subscribe(first, DragDropEffects.Copy, firstHandler, out var firstId);
        registry.Subscribe(second, DragDropEffects.Copy, secondHandler, out _);

        Assert.Equal(0, registry.Unsubscribe(firstId));
        Assert.Equal(HResults.E_INVALIDARG, registry.Unsubscribe(firstId));

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Raise(first, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Raise(second, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Dispatcher.UIThread.RunJobs();

        Assert.Empty(firstHandler.Events);
        Assert.Single(secondHandler.Events);

        registry.Clear();
        Raise(second, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Dispatcher.UIThread.RunJobs();
        Assert.Single(secondHandler.Events);
        Assert.False(DragDrop.GetAllowDrop(second));
    }

    [Fact]
    public void A_notification_queued_before_unsubscribing_is_never_delivered()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out var id);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));

        // The notification is posted, not delivered inline, so unsubscribing
        // before the dispatcher drains has to win.
        Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        Assert.Empty(handler.Events);

        Assert.Equal(0, registry.Unsubscribe(id));
        Dispatcher.UIThread.RunJobs();

        Assert.Empty(handler.Events);
    }

    [Fact]
    public void A_notification_queued_before_shutdown_is_never_delivered()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = CreateTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Raise(border, DragDrop.DropEvent, transfer, DragDropEffects.Copy);

        registry.Clear();
        Dispatcher.UIThread.RunJobs();

        Assert.Empty(handler.Events);
    }

    [Fact]
    public void The_payload_is_captured_once_per_drag_not_once_per_notification()
    {
        // DragOver fires continuously inside the platform drag loop, so the
        // payload must not be re-materialized for every notification.
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var transfer = new CountingDataTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));

        Raise(border, DragDrop.DragEnterEvent, transfer, DragDropEffects.Copy);
        var afterEnter = transfer.MaterializeCount;
        Assert.True(afterEnter > 0, "entering a drag must capture the payload");

        for (var i = 0; i < 20; i++)
            Raise(border, DragDrop.DragOverEvent, transfer, DragDropEffects.Copy);
        var drop = Raise(border, DragDrop.DropEvent, transfer, DragDropEffects.Copy);

        Assert.Equal(DragDropEffects.Copy, drop.DragEffects);
        Assert.Equal(afterEnter, transfer.MaterializeCount);

        Dispatcher.UIThread.RunJobs();
        Assert.Equal(22, handler.Events.Count);
        Assert.All(handler.Events, notification => Assert.Single(notification.Uris));
    }

    [Fact]
    public void A_new_drag_captures_a_fresh_payload()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var registry = new AvnFileDropRegistry();
        var border = new Border();
        var handler = new RecordingHandler();
        registry.Subscribe(border, DragDropEffects.Copy, handler, out _);

        using var first = new CountingDataTransfer(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));
        Raise(border, DragDrop.DragEnterEvent, first, DragDropEffects.Copy);
        Raise(border, DragDrop.DragLeaveEvent, first, DragDropEffects.Copy);

        using var second = new CountingDataTransfer(
            new FakeStorageFile("b.log", new Uri("file:///logs/b.log")));
        Raise(border, DragDrop.DragEnterEvent, second, DragDropEffects.Copy);
        Dispatcher.UIThread.RunJobs();

        Assert.True(first.MaterializeCount > 0);
        Assert.True(second.MaterializeCount > 0);
        Assert.Equal(3, handler.Events.Count);
        Assert.Equal(new[] { "file:///logs/a.log" }, handler.Events[0].Uris);
        Assert.Empty(handler.Events[1].Uris);
        Assert.Equal(new[] { "file:///logs/b.log" }, handler.Events[2].Uris);
    }

    private static DataTransfer CreateTransfer(params IStorageItem[] items)
    {
        var transfer = new DataTransfer();
        foreach (var item in items)
            transfer.Add(DataTransferItem.Create(DataFormat.File, item));
        return transfer;
    }

    private static DragEventArgs Raise(
        Interactive target,
        RoutedEvent<DragEventArgs> routedEvent,
        IDataTransfer transfer,
        DragDropEffects allowed)
    {
        var args = new DragEventArgs(routedEvent, transfer, target, default, KeyModifiers.None)
        {
            DragEffects = allowed,
            Source = target,
        };
        target.RaiseEvent(args);
        return args;
    }

    /// <summary>
    /// Counts how often the payload is materialized, so a regression that
    /// re-captures inside the drag loop is visible.
    /// </summary>
    private sealed class CountingDataTransfer : IDataTransfer
    {
        private readonly IDataTransferItem[] _items;

        public CountingDataTransfer(params IStorageItem[] items) =>
            _items = items
                .Select(item => (IDataTransferItem)DataTransferItem.Create(DataFormat.File, item))
                .ToArray();

        public int MaterializeCount { get; private set; }

        public IReadOnlyList<DataFormat> Formats { get; } = [DataFormat.File];

        public IReadOnlyList<IDataTransferItem> Items
        {
            get
            {
                MaterializeCount++;
                return _items;
            }
        }

        public void Dispose()
        {
        }
    }

    private sealed record Notification(
        long SubscriptionId,
        int Kind,
        int AllowedEffects,
        int EffectiveEffects,
        IReadOnlyList<string> Uris);

    private sealed class RecordingHandler : IAvnFileDropHandler
    {
        public List<Notification> Events { get; } = new();

        public int OnDragEvent(
            long subscriptionId,
            int kind,
            int allowedEffects,
            int effectiveEffects,
            IAvnStorageItemList? items)
        {
            var uris = new List<string>();
            if (items is not null)
            {
                items.GetCount(out var count);
                for (var i = 0; i < count; i++)
                {
                    items.GetItem(i, out var item);
                    item!.GetUri(out var uri);
                    uris.Add(uri!);
                }
            }

            Events.Add(new Notification(subscriptionId, kind, allowedEffects, effectiveEffects, uris));
            return 0;
        }
    }

    private sealed class FailingHandler : IAvnFileDropHandler
    {
        public int CallCount { get; private set; }

        public int OnDragEvent(
            long subscriptionId,
            int kind,
            int allowedEffects,
            int effectiveEffects,
            IAvnStorageItemList? items)
        {
            CallCount++;
            return HResults.E_FAIL;
        }
    }
}
