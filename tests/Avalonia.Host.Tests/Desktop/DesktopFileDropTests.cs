using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Host.Desktop;
using Avalonia.Input;
using Avalonia.Platform.Storage;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

/// <summary>
/// Incoming drag-and-drop negotiation. Rust is never consulted synchronously:
/// the effect is the intersection of the platform's allowed effects and the
/// conservative mask declared at subscription time.
/// </summary>
public class DesktopFileDropTests
{
    [Theory]
    [InlineData(DragDropEffects.Copy | DragDropEffects.Move, DragDropEffects.Copy, DragDropEffects.Copy)]
    [InlineData(DragDropEffects.Move, DragDropEffects.Copy, DragDropEffects.None)]
    [InlineData(DragDropEffects.Copy | DragDropEffects.Link, DragDropEffects.Copy | DragDropEffects.Link, DragDropEffects.Copy | DragDropEffects.Link)]
    [InlineData(DragDropEffects.None, DragDropEffects.Copy, DragDropEffects.None)]
    public void The_effect_is_the_intersection_of_allowed_and_accepted(
        DragDropEffects allowed,
        DragDropEffects accepted,
        DragDropEffects expected) =>
        Assert.Equal(expected, DesktopFileDrop.NegotiateEffect(allowed, accepted, hasItems: true));

    [Fact]
    public void A_payload_without_files_is_always_refused()
    {
        Assert.Equal(
            DragDropEffects.None,
            DesktopFileDrop.NegotiateEffect(
                DragDropEffects.Copy | DragDropEffects.Move,
                DragDropEffects.Copy,
                hasItems: false));
    }

    [Fact]
    public void Items_are_captured_from_the_payload_while_it_is_still_valid()
    {
        using var transfer = FileDataTransfer.Create(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")),
            new FakeStorageFolder("logs", new Uri("file:///logs/")));

        var items = DesktopFileDrop.CaptureItems(transfer);

        Assert.Equal(2, items.Count);
        Assert.Equal(
            new[] { "file:///logs/a.log", "file:///logs/" },
            items.Select(item => item.Uri));
        Assert.True(items[1].IsFolder);
    }

    [Fact]
    public void A_payload_with_no_file_format_captures_nothing()
    {
        using var transfer = FileDataTransfer.Create();

        Assert.Empty(DesktopFileDrop.CaptureItems(transfer));
        Assert.Empty(DesktopFileDrop.CaptureItems(null));
    }

    [Fact]
    public void Enter_over_and_drop_carry_items_while_leave_does_not()
    {
        using var transfer = FileDataTransfer.Create(
            new FakeStorageFile("a.log", new Uri("file:///logs/a.log")));

        foreach (var kind in new[]
                 {
                     DesktopDropEventKind.Enter,
                     DesktopDropEventKind.Over,
                     DesktopDropEventKind.Drop,
                 })
        {
            var notification = DesktopFileDrop.Prepare(
                kind,
                DragDropEffects.Copy | DragDropEffects.Move,
                DragDropEffects.Copy,
                transfer);
            Assert.Equal(kind, notification.Kind);
            Assert.Single(notification.Items);
            Assert.Equal(DragDropEffects.Copy, notification.EffectiveEffects);
        }

        var leave = DesktopFileDrop.Prepare(
            DesktopDropEventKind.Leave,
            DragDropEffects.Copy,
            DragDropEffects.Copy,
            transfer);
        Assert.Empty(leave.Items);
        Assert.Equal(DragDropEffects.None, leave.EffectiveEffects);
    }

    [Fact]
    public void A_non_local_dropped_item_keeps_its_uri()
    {
        using var transfer = FileDataTransfer.Create(
            new FakeStorageFile("doc", new Uri("content://media/documents/7")));

        var notification = DesktopFileDrop.Prepare(
            DesktopDropEventKind.Drop,
            DragDropEffects.Copy,
            DragDropEffects.Copy,
            transfer);

        var item = Assert.Single(notification.Items);
        Assert.Equal("content://media/documents/7", item.Uri);
        Assert.Null(item.LocalPath);
    }

    private static class FileDataTransfer
    {
        public static DataTransfer Create(params IStorageItem[] items)
        {
            var transfer = new DataTransfer();
            foreach (var item in items)
                transfer.Add(DataTransferItem.Create(DataFormat.File, item));
            return transfer;
        }
    }
}
