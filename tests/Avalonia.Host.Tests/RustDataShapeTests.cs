using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.Linq;
using Avalonia.Rust;
using Avalonia.Rust.Interop;
using Avalonia.Rust.Sample.Generated;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Stage 30 tests. The load-bearing assertions here are the *counts*: a
/// 100,000-row window must report 100,000 while keeping the number of live
/// managed row adapters bounded, and every rejected range must release exactly
/// the adapters it staged.
/// </summary>
public class RustDataShapeTests
{
    private const int InvalidArgument = unchecked((int)0x80070057);

    [Fact]
    public void Windowed_collection_reports_the_full_dataset_but_realizes_bounded_adapters()
    {
        var source = new FakeRangeSource(generation: 7, totalCount: 100_000);
        using var window = new RustWindowedCollection(5, 64, 8, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(7, 100_000);

        Assert.Equal(100_000, window.Count);
        Assert.Equal(0, window.LiveElementCount);

        // Walk the whole dataset one page at a time, answering every request.
        for (var index = 0; index < 100_000; index += 64)
        {
            Assert.Null(window[index]);
            Assert.True(source.TryAnswer(window));
            Assert.NotNull(window[index]);
        }

        Assert.Equal(100_000, window.Count);
        // The final page is a partial one (100,000 is not a multiple of 64),
        // so the live budget is 7 full pages plus that remainder.
        Assert.Equal((7 * 64) + (100_000 % 64), window.LiveElementCount);
        Assert.True(window.LiveElementCount < 1000, "live adapters must stay far below the dataset size");
        Assert.True(window.DetachedElementCount > 90_000, "evicted pages must actually detach their adapters");
        Assert.Equal(TrackedRow.Created - TrackedRow.Disposed, window.LiveElementCount);
    }

    [Fact]
    public void Enumerating_a_windowed_collection_never_realizes_anything()
    {
        var source = new FakeRangeSource(generation: 1, totalCount: 100_000);
        using var window = new RustWindowedCollection(5, 64, 8, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(1, 100_000);

        var materialized = 0;
        foreach (var item in window)
        {
            if (item is not null)
                materialized++;
        }

        Assert.Equal(0, materialized);
        Assert.Empty(source.Requests);
    }

    [Fact]
    public void A_stale_range_generation_is_rejected_without_realizing_or_leaking_adapters()
    {
        var source = new FakeRangeSource(generation: 3, totalCount: 512);
        using var window = new RustWindowedCollection(5, 64, 8, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(3, 512);

        var created = TrackedRow.Created;
        var stale = new TrackedRow("stale");
        var applied = window.ApplyRange(2, 512, 0, [stale]);

        Assert.False(applied);
        stale.Dispose();
        Assert.Equal(0, window.LiveElementCount);
        Assert.Equal(created + 1, TrackedRow.Created);

        // Mismatched totals and misaligned offsets are equally rejected. The window
        // refuses ownership, so the caller owns the rows: dispose them so the static
        // Created/Disposed counters stay balanced for tests that run after this one.
        var rejected = new[]
        {
            new TrackedRow("wrong total"),
            new TrackedRow("misaligned"),
            new TrackedRow("short page"),
        };
        Assert.False(window.ApplyRange(3, 511, 0, [rejected[0]]));
        Assert.False(window.ApplyRange(3, 512, 5, [rejected[1]]));
        Assert.False(window.ApplyRange(3, 512, 0, [rejected[2]]));
        Assert.Equal(0, window.LiveElementCount);
        foreach (var row in rejected)
            row.Dispose();
    }

    [Fact]
    public void A_new_generation_detaches_every_realized_page()
    {
        var source = new FakeRangeSource(generation: 1, totalCount: 256);
        using var window = new RustWindowedCollection(5, 64, 8, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(1, 256);

        Assert.Null(window[0]);
        Assert.True(source.TryAnswer(window));
        Assert.Equal(64, window.LiveElementCount);

        var resets = 0;
        window.CollectionChanged += (_, e) =>
        {
            if (e.Action == NotifyCollectionChangedAction.Reset)
                resets++;
        };
        var detached = window.DetachedElementCount;
        source.Advance(generation: 2, totalCount: 300);
        window.ResetTo(2, 300);

        Assert.Equal(1, resets);
        Assert.Equal(300, window.Count);
        Assert.Equal(0, window.LiveElementCount);
        Assert.Equal(detached + 64, window.DetachedElementCount);
    }

    [Fact]
    public void The_range_coordinator_applies_fills_and_rejects_stale_batches()
    {
        var pending = new List<Action>();
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.ResetTo(9, 8);
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        var fresh = new FakeRangeBatch(5, generation: 9, total: 8, offset: 0, ["a", "b", "c", "d"]);
        Assert.Equal(0, coordinator.Publish(fresh));
        Assert.Single(pending);
        Assert.Equal(0, window.LiveElementCount);

        pending[0]();
        Assert.Equal(RustVmBatchOutcome.Applied, fresh.Outcome);
        Assert.Equal(4, window.LiveElementCount);
        Assert.Equal("a", ((TrackedRow)window[0]!).Text);

        var created = TrackedRow.Created;
        var stale = new FakeRangeBatch(5, generation: 8, total: 8, offset: 4, ["e", "f", "g", "h"]);
        coordinator.Publish(stale);
        pending[1]();

        Assert.Equal(RustVmBatchOutcome.Stale, stale.Outcome);
        Assert.Equal(created, TrackedRow.Created);
        Assert.Equal(4, window.LiveElementCount);

        var unknown = new FakeRangeBatch(6, generation: 9, total: 8, offset: 0, ["x"]);
        coordinator.Publish(unknown);
        pending[2]();
        Assert.Equal(RustVmBatchOutcome.Error, unknown.Outcome);

        coordinator.Close();
        var cancelled = new FakeRangeBatch(5, generation: 9, total: 8, offset: 4, ["e", "f", "g", "h"]);
        Assert.Equal(0, coordinator.Publish(cancelled));
        pending[3]();
        Assert.Equal(RustVmBatchOutcome.Cancelled, cancelled.Outcome);
        Assert.Equal(InvalidArgument, coordinator.Publish(null));
    }

    [Fact]
    public void Realization_counters_notify_so_a_bound_view_can_show_the_bound()
    {
        var source = new FakeRangeSource(generation: 1, totalCount: 128);
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(1, 128);
        var changed = new List<string?>();
        window.PropertyChanged += (_, e) => changed.Add(e.PropertyName);

        Assert.Null(window[0]);
        Assert.True(source.TryAnswer(window));

        Assert.Equal(4, window.LiveElementCount);
        Assert.Contains(nameof(window.LiveElementCount), changed);
        Assert.Contains(nameof(window.DetachedElementCount), changed);
        Assert.Contains(nameof(window.ViewportStart), changed);
    }

    [Fact]
    public void Stage_thirty_interfaces_have_their_published_slot_counts()
    {
        // Managed [GeneratedComInterface] dispatch is purely positional, so a
        // method added on one side only would silently shift every later slot
        // and corrupt arguments. The Rust vtable structs assert the same
        // counts in `rust_vm::tests::stage_thirty_vtables_have_their_published_slot_counts`.
        Assert.Equal(11, typeof(IAvnRustVmSink4).GetMethods().Length);
        Assert.Equal(10, typeof(IAvnRustVmRangeBatch).GetMethods().Length);
        Assert.Equal(2, typeof(IAvnRustRangeSource).GetMethods().Length);
        Assert.Equal(2, typeof(IAvnRustViewModel2).GetMethods().Length);
        Assert.Equal(6, typeof(IAvnRustVmSink5).GetMethods().Length);

        // Slot order is ABI: GetKind must stay first on the range batch, as it
        // is on IAvnRustVmUpdateOperation.
        Assert.Equal("GetKind", typeof(IAvnRustVmRangeBatch).GetMethods()[0].Name);
        Assert.Equal("Complete", typeof(IAvnRustVmRangeBatch).GetMethods()[^1].Name);
        Assert.Equal("AddInteger", typeof(IAvnRustVmSink5).GetMethods()[0].Name);
        Assert.Equal("ReplaceDouble", typeof(IAvnRustVmSink5).GetMethods()[^1].Name);
    }

    [Fact]
    public void Generated_adapter_projects_a_double_collection_through_the_number_sink()
    {
        var model = new ShapeModel();
        using var adapter = new SampleViewModelAdapter(model, action => action(), action => action());
        var changed = new List<NotifyCollectionChangedAction>();
        adapter.CoreLoads.CollectionChanged += (_, e) => changed.Add(e.Action);

        var sink = model.Sink5;
        Assert.Equal(0, sink.AddDouble(8, 0.25));
        Assert.Equal(0, sink.AddDouble(8, 0.75));
        Assert.Equal(0, sink.InsertDouble(8, 1, 0.5));
        Assert.Equal([0.25, 0.5, 0.75], adapter.CoreLoads.ToArray());

        Assert.Equal(0, sink.ReplaceDouble(8, 0, 1.5));
        Assert.Equal([1.5, 0.5, 0.75], adapter.CoreLoads.ToArray());

        // Out-of-range indices and mismatched element kinds are explicit ABI
        // errors, never a silent coercion or a partially applied edit.
        Assert.Equal(InvalidArgument, sink.InsertDouble(8, 9, 1.0));
        Assert.Equal(InvalidArgument, sink.ReplaceDouble(8, 3, 1.0));
        Assert.Equal(InvalidArgument, sink.AddInteger(8, 1));
        Assert.Equal(InvalidArgument, sink.AddDouble(1, 1.0));
        Assert.Equal([1.5, 0.5, 0.75], adapter.CoreLoads.ToArray());

        // Removal, movement and clearing carry no element value, so they ride
        // the already-published v2 sink.
        var v2 = model.Sink2;
        Assert.Equal(0, v2.MoveItem(8, 0, 2));
        Assert.Equal([0.5, 0.75, 1.5], adapter.CoreLoads.ToArray());
        Assert.Equal(0, v2.RemoveAt(8, 1));
        Assert.Equal([0.5, 1.5], adapter.CoreLoads.ToArray());
        Assert.Equal(InvalidArgument, v2.RemoveAt(8, 5));
        Assert.Equal(0, v2.ClearCollection(8));
        Assert.Empty(adapter.CoreLoads);
        Assert.Equal(NotifyCollectionChangedAction.Reset, changed[^1]);
    }

    [Fact]
    public void Generated_adapter_projects_an_integer_collection_through_the_number_sink()
    {
        var model = new ShapeModel();
        using var adapter = new SampleViewModelAdapter(model, action => action(), action => action());
        var sink = model.Sink5;

        Assert.Equal(0, sink.AddInteger(9, 10));
        Assert.Equal(0, sink.AddInteger(9, 30));
        Assert.Equal(0, sink.InsertInteger(9, 1, 20));
        Assert.Equal([10L, 20L, 30L], adapter.CoreTicks.ToArray());

        Assert.Equal(0, sink.ReplaceInteger(9, 2, 99));
        Assert.Equal([10L, 20L, 99L], adapter.CoreTicks.ToArray());

        Assert.Equal(InvalidArgument, sink.InsertInteger(9, 4, 1));
        Assert.Equal(InvalidArgument, sink.ReplaceInteger(9, 3, 1));
        // The integer and double collections are distinct schema IDs, so a
        // call must never land on the other one's list.
        Assert.Equal(InvalidArgument, sink.AddDouble(9, 1.0));
        Assert.Equal(InvalidArgument, sink.AddInteger(8, 1));
        Assert.Equal([10L, 20L, 99L], adapter.CoreTicks.ToArray());
        Assert.Empty(adapter.CoreLoads);

        Assert.Equal(0, model.Sink2.ClearCollection(9));
        Assert.Empty(adapter.CoreTicks);
    }

    [Fact]
    public void A_string_collection_rejects_the_number_sink_and_keeps_its_own_transport()
    {
        var model = new ShapeModel();
        using var adapter = new SampleViewModelAdapter(model, action => action(), action => action());

        var numbers = model.Sink5;
        var v1 = model.Sink;

        Assert.Equal(InvalidArgument, numbers.AddDouble(1, 1.0));
        Assert.Equal(InvalidArgument, numbers.AddInteger(1, 1));
        Assert.Equal(0, v1.AddString(1, "kept"));
        Assert.Equal(["kept"], adapter.Items.ToArray());
        Assert.Empty(adapter.CoreLoads);
    }

    [Fact]
    public void A_reset_batch_republishes_the_dataset_identity()
    {
        var pending = new List<Action>();
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        var reset = new FakeRangeBatch(5, generation: 12, total: 40, offset: 0, [], RustRangeCoordinator.RangeReset);
        coordinator.Publish(reset);
        pending[0]();

        Assert.Equal(RustVmBatchOutcome.Applied, reset.Outcome);
        Assert.Equal(12, window.Generation);
        Assert.Equal(40, window.Count);

        var unknownKind = new FakeRangeBatch(5, generation: 12, total: 40, offset: 0, [], kind: 7);
        coordinator.Publish(unknownKind);
        pending[1]();
        Assert.Equal(RustVmBatchOutcome.Error, unknownKind.Outcome);
    }

    [Fact]
    public void An_unrecognized_batch_kind_leaves_the_page_requestable()
    {
        var pending = new List<Action>();
        var source = new FakeRangeSource(generation: 3, totalCount: 16);
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(3, 16);
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        Assert.Null(window[0]);
        Assert.Single(source.Requests);

        // A producer newer than this host answers with a kind it does not
        // know; the page must degrade to "ask again", not to "blank forever".
        var future = new FakeRangeBatch(5, generation: 3, total: 16, offset: 0, [], kind: 9);
        coordinator.Publish(future);
        pending[0]();

        Assert.Equal(RustVmBatchOutcome.Error, future.Outcome);
        Assert.Null(window[0]);
        Assert.Equal(2, source.Requests.Count);
    }

    [Fact]
    public void Disposing_a_window_with_a_throwing_observer_still_reports_disposed()
    {
        var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.ResetTo(1, 16);
        window.CollectionChanged += (_, _) => throw new InvalidOperationException("observer");

        // The generated adapter disposes a window through TryDispose, so a
        // hostile observer cannot abort the rest of the teardown.
        Assert.Throws<InvalidOperationException>(window.Dispose);
        Assert.Empty(window);
    }

    [Fact]
    public void A_page_is_requestable_again_after_a_rejected_or_evicted_request()
    {
        var pending = new List<Action>();
        var source = new FakeRangeSource(generation: 2, totalCount: 16);
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(2, 16);
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        // An old answer must not remove the current generation's pending request.
        Assert.Null(window[0]);
        Assert.Single(source.Requests);
        Assert.Null(window[0]);
        Assert.Single(source.Requests);

        var stale = new FakeRangeBatch(5, generation: 1, total: 16, offset: 0, ["a", "b", "c", "d"]);
        coordinator.Publish(stale);
        pending[0]();
        Assert.Equal(RustVmBatchOutcome.Stale, stale.Outcome);

        Assert.Null(window[0]);
        Assert.Single(source.Requests);
        Assert.False(window.ApplyRange(1, 16, 0, Array.Empty<object?>()));
        Assert.Null(window[0]);
        Assert.Single(source.Requests);

        window.AbandonPage(2, 0);
        Assert.Null(window[0]);
        Assert.Equal(2, source.Requests.Count);

        // Rust's bounded queue evicts the oldest request; the reported offset
        // must free that page rather than leaving it waiting forever.
        source.Requests.Clear();
        source.Dropped = (5, 0);
        Assert.Null(window[8]);
        Assert.Single(source.Requests);
        Assert.Null(window[0]);
        Assert.Equal(2, source.Requests.Count);

        // The queue is shared by every windowed collection on a model, so an
        // eviction belonging to a *different* collection must not un-pend one
        // of this collection's live pages.
        while (source.TryAnswer(window))
        {
        }
        source.Requests.Clear();
        source.Dropped = (6, 4);
        Assert.Null(window[4]);
        Assert.Single(source.Requests);
        Assert.Null(window[4]);
        Assert.Single(source.Requests);
    }

    [Fact]
    public void EnsureVisibleRange_requests_overlapping_pages_without_waiting_on_indexer_misses()
    {
        var source = new FakeRangeSource(generation: 1, totalCount: 256);
        using var window = new RustWindowedCollection(5, 64, 8, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(1, 256);

        window.EnsureVisibleRange(0, 70);
        Assert.Equal(2, source.Requests.Count);
        Assert.Equal(0L, source.Requests[0].Offset);
        Assert.Equal(64L, source.Requests[1].Offset);

        window.EnsureVisibleRange(0, 70);
        Assert.Equal(2, source.Requests.Count);
    }

    [Fact]
    public void Refreshing_realized_pages_reissues_range_requests_without_dropping_identity()
    {
        var source = new FakeRangeSource(generation: 4, totalCount: 16);
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(4, 16);

        Assert.Null(window[0]);
        Assert.True(source.TryAnswer(window));
        Assert.Equal(4, window.LiveElementCount);
        Assert.Equal("row-0", ((TrackedRow)window[0]!).Text);

        var requests = source.Requests.Count;
        var detached = window.DetachedElementCount;
        window.RefreshRealized();
        Assert.Equal(requests + 1, source.Requests.Count);
        Assert.True(source.TryAnswer(window));
        Assert.Equal(4, window.Generation);
        Assert.Equal(16, window.Count);
        Assert.Equal(detached + 4, window.DetachedElementCount);
        Assert.Equal(4, window.LiveElementCount);
        Assert.Equal("row-0", ((TrackedRow)window[0]!).Text);
    }

    [Fact]
    public void A_range_invalidate_batch_refreshes_realized_pages_and_rejects_stale_generations()
    {
        var pending = new List<Action>();
        var source = new FakeRangeSource(generation: 2, totalCount: 8);
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.SetSource(source);
        window.ResetTo(2, 8);
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        Assert.Null(window[0]);
        Assert.True(source.TryAnswer(window));
        source.Requests.Clear();

        var stale = new FakeRangeBatch(5, generation: 1, total: 8, offset: 0, [], kind: RustRangeCoordinator.RangeInvalidate);
        coordinator.Publish(stale);
        pending[^1]();
        Assert.Equal(RustVmBatchOutcome.Stale, stale.Outcome);
        Assert.Empty(source.Requests);

        var fresh = new FakeRangeBatch(5, generation: 2, total: 8, offset: 0, [], kind: RustRangeCoordinator.RangeInvalidate);
        coordinator.Publish(fresh);
        pending[^1]();
        Assert.Equal(RustVmBatchOutcome.Applied, fresh.Outcome);
        Assert.Single(source.Requests);
        Assert.Equal(0L, source.Requests[0].Offset);
    }

    [Fact]
    public void A_shared_queue_eviction_unpends_the_victim_collection()
    {
        var source = new FakeRangeSource(generation: 1, totalCount: 16);
        using var victim = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        using var requester = new RustWindowedCollection(6, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        victim.SetSource(source);
        requester.SetSource(source);
        victim.ResetTo(1, 16);
        requester.ResetTo(1, 16);
        RustWindowedCollection? Resolve(int id) => id == 5 ? victim : id == 6 ? requester : null;
        victim.SetPeerResolver(Resolve);
        requester.SetPeerResolver(Resolve);

        Assert.Null(victim[0]);
        Assert.Single(source.Requests);

        source.Dropped = (5, 0);
        Assert.Null(requester[4]);
        Assert.Equal(2, source.Requests.Count);

        source.Requests.Clear();
        Assert.Null(victim[0]);
        Assert.Single(source.Requests);
        Assert.Equal(5, source.Requests[0].Collection);
        Assert.Equal(0L, source.Requests[0].Offset);
    }

    [Fact]
    public void Disposing_a_window_publishes_the_emptied_count()
    {
        var window = new RustWindowedCollection(5, 4, 2, (_, text) => new TrackedRow(text ?? ""));
        window.ResetTo(1, 16);
        var resets = 0;
        window.CollectionChanged += (_, e) =>
        {
            if (e.Action == NotifyCollectionChangedAction.Reset)
                resets++;
        };

        window.Dispose();

        Assert.Equal(1, resets);
        Assert.Empty(window);
    }

    [Fact]
    public void A_failing_element_factory_releases_the_model_and_every_staged_element()
    {
        var pending = new List<Action>();
        var created = 0;
        using var window = new RustWindowedCollection(5, 4, 2, (_, text) =>
        {
            if (++created == 3)
                throw new InvalidOperationException("attach failed");
            return new TrackedRow(text ?? "");
        });
        window.ResetTo(1, 4);
        var coordinator = new RustRangeCoordinator(id => id == 5 ? window : null, pending.Add);

        var disposed = TrackedRow.Disposed;
        var batch = new FakeRangeBatch(5, generation: 1, total: 4, offset: 0, ["a", "b", "c", "d"]);
        coordinator.Publish(batch);
        pending[0]();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal(disposed + 2, TrackedRow.Disposed);
        Assert.Equal(0, window.LiveElementCount);
    }

    [Fact]
    public void Observable_map_publishes_incremental_insert_replace_remove_and_clear()
    {
        var map = new RustObservableMap<string, long>();
        var actions = new List<NotifyCollectionChangedAction>();
        var properties = new List<string?>();
        map.CollectionChanged += (_, e) => actions.Add(e.Action);
        map.PropertyChanged += (_, e) => properties.Add(e.PropertyName);

        Assert.True(map.Set("Error", 1, out _));
        Assert.True(map.Set("Warning", 2, out _));
        Assert.True(map.Set("Error", 5, out var displaced));
        Assert.Equal(1, displaced);
        Assert.False(map.Set("Error", 5, out _));
        Assert.True(map.Remove("Warning", out var removed));
        Assert.Equal(2, removed);
        Assert.False(map.Remove("Missing", out _));

        Assert.Equal(
            [
                NotifyCollectionChangedAction.Add,
                NotifyCollectionChangedAction.Add,
                NotifyCollectionChangedAction.Replace,
                NotifyCollectionChangedAction.Remove,
            ],
            actions);
        Assert.Equal(5, map["Error"]);
        Assert.Single(map);
        Assert.Contains("Count", properties);

        var cleared = map.Clear();
        Assert.Equal([5L], cleared);
        Assert.Equal(NotifyCollectionChangedAction.Reset, actions[^1]);
        Assert.Empty(map);
        Assert.Empty(map.Clear());
    }

    [Fact]
    public void Observable_map_keeps_insertion_order_after_a_middle_removal()
    {
        var map = new RustObservableMap<long, string>();
        map.Set(1, "one", out _);
        map.Set(2, "two", out _);
        map.Set(3, "three", out _);

        map.Remove(2, out _);
        map.Set(4, "four", out _);

        Assert.Equal([1L, 3L, 4L], map.Keys.ToArray());
        Assert.Equal("three", map[3L]);
        Assert.Equal(new KeyValuePair<long, string>(4, "four"), ((IReadOnlyList<KeyValuePair<long, string>>)map)[2]);
    }

    [Fact]
    public void Generated_adapter_projects_maps_trees_windows_progress_and_results()
    {
        var model = new ShapeModel();
        using var adapter = new SampleViewModelAdapter(model, action => action(), action => action());

        Assert.Equal(2, model.Sink4Queries);
        Assert.Equal(100, adapter.LogWindow.Count);
        Assert.Equal("Generation 4", adapter.LogWindowStatus);

        var sink = model.Sink4;
        Assert.Equal(0, sink.MapSetInteger(1, "Error", 0, 12));
        Assert.Equal(12, adapter.SeverityCounts["Error"]);
        Assert.Equal(0, sink.MapSetModel(2, "CMTrace.0", 0, new BareModel()));
        Assert.Single(adapter.SourceDetails);
        Assert.Equal(0, sink.MapRemove(2, "CMTrace.0", 0));
        Assert.Empty(adapter.SourceDetails);
        Assert.Equal(InvalidArgument, sink.MapSetInteger(99, "x", 0, 1));

        Assert.Equal(0, sink.SetCommandRunning(3, 1));
        Assert.True(adapter.SaveIsRunning);
        Assert.True(adapter.CancelSaveCommand.CanExecute(null));
        Assert.Equal(0, sink.SetCommandProgress(3, 1, 3.5, "clamped"));
        Assert.Equal(1.0, adapter.SaveProgress);
        Assert.Equal("clamped", adapter.SaveProgressMessage);
        Assert.Equal(0, sink.SetCommandProgress(3, 0, 0.5, "indeterminate"));
        Assert.Null(adapter.SaveProgress);

        Assert.Equal(0, sink.SetCommandResult(3, new BareModel()));
        Assert.NotNull(adapter.SaveResult);
        Assert.Equal(0, sink.SetCommandRunning(3, 0));
        Assert.False(adapter.SaveIsRunning);
        Assert.False(adapter.CancelSaveCommand.CanExecute(null));
        Assert.Null(adapter.SaveProgressMessage);

        adapter.SaveCommand.Execute(null);
        Assert.Equal(1, model.TrackedStarts);
        adapter.CancelSaveCommand.Execute(null);
        Assert.Equal([(3, 1L)], model.Cancellations);
    }

    [Fact]
    public void Generated_metadata_describes_windows_trees_maps_and_command_shapes()
    {
        var descriptor = SampleViewModelMetadata.Descriptor;

        var window = descriptor.Collections.Single(collection => collection.Name == "LogWindow");
        Assert.NotNull(window.Window);
        Assert.Equal(64, window.Window!.PageSize);
        Assert.Equal(8, window.Window.MaxLivePages);

        var tree = descriptor.Collections.Single(collection => collection.Name == "LogTree");
        Assert.NotNull(tree.Tree);
        Assert.Equal("Children", tree.Tree!.ChildrenCollection);
        Assert.Equal("Label", tree.Tree.HeaderPath);
        Assert.Equal("HasChildren", tree.Tree.HasChildrenProperty);

        var children = LogNodeViewModelMetadata.Descriptor.Collections.Single();
        Assert.True(children.Recursive);
        Assert.Null(children.ElementDescriptor);

        Assert.Equal(2, descriptor.Maps.Count);
        Assert.Equal(RustViewModelValueKind.String, descriptor.Maps[0].KeyKind);
        Assert.Equal(RustViewModelValueKind.Integer, descriptor.Maps[0].ValueKind);
        Assert.Equal("TraceEventViewModel", descriptor.Maps[1].ValueDescriptor!.Name);

        var save = descriptor.Commands.Single(command => command.Name == "SaveCommand");
        Assert.True(save.SupportsProgress);
        Assert.True(save.SupportsCancellation);
        Assert.Equal("SaveReportViewModel", save.ResultDescriptor!.Name);
    }

    private sealed class TrackedRow(string text) : IDisposable
    {
        public static int Created { get; private set; }

        public static int Disposed { get; private set; }

        public string Text { get; } = Track(text);

        private static string Track(string text)
        {
            Created++;
            return text;
        }

        public void Dispose() => Disposed++;
    }

    /// <summary>Answers range requests synchronously, one at a time, on demand.</summary>
    private sealed class FakeRangeSource(long generation, long totalCount) : IAvnRustRangeSource
    {
        public List<(int Collection, long Offset, int Length, long Generation)> Requests { get; } = [];

        public void Advance(long generation, long totalCount)
        {
            Generation = generation;
            TotalCount = totalCount;
            Requests.Clear();
        }

        public long Generation { get; private set; } = generation;

        public long TotalCount { get; private set; } = totalCount;

        public int GetRangeState(int collectionId, out long generation, out long total)
        {
            generation = Generation;
            total = TotalCount;
            return 0;
        }

        public (int Collection, long Offset) Dropped { get; set; } = (0, -1);

        public int RequestRange(
            int collectionId,
            long offset,
            int length,
            long generation,
            out int droppedCollectionId,
            out long droppedOffset)
        {
            Requests.Add((collectionId, offset, length, generation));
            (droppedCollectionId, droppedOffset) = Dropped;
            Dropped = (0, -1);
            return 0;
        }

        public bool TryAnswer(RustWindowedCollection window)
        {
            if (Requests.Count == 0)
                return false;
            var (_, offset, length, generation) = Requests[0];
            Requests.RemoveAt(0);
            var items = Enumerable
                .Range(0, length)
                .Select(index => (object?)new TrackedRow($"row-{offset + index}"))
                .ToArray();
            return window.ApplyRange(generation, TotalCount, offset, items);
        }
    }

    private sealed class FakeRangeBatch(
        int collectionId,
        long generation,
        long total,
        long offset,
        string[] items,
        int kind = RustRangeCoordinator.RangeFill)
        : IAvnRustVmRangeBatch
    {
        public RustVmBatchOutcome? Outcome { get; private set; }

        public int GetKind(out int value)
        {
            value = kind;
            return 0;
        }

        public int GetCollectionId(out int value)
        {
            value = collectionId;
            return 0;
        }

        public int GetGeneration(out long value)
        {
            value = generation;
            return 0;
        }

        public int GetTotalCount(out long value)
        {
            value = total;
            return 0;
        }

        public int GetOffset(out long value)
        {
            value = offset;
            return 0;
        }

        public int GetItemCount(out int value)
        {
            value = items.Length;
            return 0;
        }

        public int GetItemModel(int index, out IAvnRustViewModel? model)
        {
            model = null;
            return 0;
        }

        public int GetItemStringLength(int index, out int length)
        {
            length = items[index].Length;
            return 0;
        }

        public unsafe int CopyItemString(int index, char* destination, int capacity)
        {
            var value = items[index];
            if (capacity < value.Length + 1)
                return InvalidArgument;
            for (var i = 0; i < value.Length; i++)
                destination[i] = value[i];
            destination[value.Length] = '\0';
            return 0;
        }

        public int Complete(int outcome, int error)
        {
            Outcome = (RustVmBatchOutcome)outcome;
            return 0;
        }
    }

    /// <summary>
    /// A producer implementing the stage 30 capabilities alongside the
    /// pre-existing view-model interface, exactly as the Rust object does.
    /// </summary>
    private sealed class ShapeModel : IAvnRustViewModel, IAvnRustViewModel2, IAvnRustRangeSource
    {
        private long _nextOperation = 1;

        public IAvnRustVmSink4 Sink4 { get; private set; } = null!;

        /// <summary>The same adapter, queried for the scalar-number capability.</summary>
        public IAvnRustVmSink5 Sink5 { get; private set; } = null!;

        /// <summary>The same adapter, on its already-published v1/v2 vtables.</summary>
        public IAvnRustVmSink Sink { get; private set; } = null!;

        public IAvnRustVmSink2 Sink2 { get; private set; } = null!;

        public int Sink4Queries { get; private set; }

        public int TrackedStarts { get; private set; }

        public List<(int Command, long Operation)> Cancellations { get; } = [];

        public int Attach(IAvnRustVmSink? sink)
        {
            Sink4Queries++;
            Sink = sink!;
            Sink2 = (IAvnRustVmSink2)sink!;
            Sink4 = (IAvnRustVmSink4)sink!;
            Sink5 = (IAvnRustVmSink5)sink!;
            sink!.SetString(15, "Generation 4");
            return 0;
        }

        public int Detach() => 0;

        public int SetString(int propertyId, string? value) => 0;

        public int SetInteger(int propertyId, long value) => 0;

        public int SetBoolean(int propertyId, int value) => 0;

        public int SetDouble(int propertyId, double value) => 0;

        public int Execute(int commandId, string? parameter) => 0;

        public int BeginAsync(int commandId, string? parameter) => 0;

        public int BeginAsyncTracked(int commandId, string? parameter, out long operationId)
        {
            TrackedStarts++;
            operationId = _nextOperation++;
            return 0;
        }

        public int CancelAsync(int commandId, long operationId)
        {
            Cancellations.Add((commandId, operationId));
            return 0;
        }

        public int GetRangeState(int collectionId, out long generation, out long total)
        {
            Sink4Queries++;
            generation = 4;
            total = collectionId == 5 ? 100 : 0;
            return 0;
        }

        public int RequestRange(
            int collectionId,
            long offset,
            int length,
            long generation,
            out int droppedCollectionId,
            out long droppedOffset)
        {
            droppedCollectionId = 0;
            droppedOffset = -1;
            return 0;
        }
    }

    /// <summary>A nested producer with no behaviour, used as a map/result value.</summary>
    private sealed class BareModel : IAvnRustViewModel
    {
        public int Attach(IAvnRustVmSink? sink) => 0;

        public int Detach() => 0;

        public int SetString(int propertyId, string? value) => 0;

        public int SetInteger(int propertyId, long value) => 0;

        public int SetBoolean(int propertyId, int value) => 0;

        public int SetDouble(int propertyId, double value) => 0;

        public int Execute(int commandId, string? parameter) => 0;

        public int BeginAsync(int commandId, string? parameter) => 0;
    }
}

