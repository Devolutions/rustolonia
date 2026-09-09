using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Linq;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Primitives;
using Avalonia.Controls.Templates;
using Avalonia.Data;
using Avalonia.Rust;
using Avalonia.Rust.Interop;
using Avalonia.Rust.Sample.Generated;
using Avalonia.Styling;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the shared staged transactional batch engine
/// (<see cref="RustVmBatchCoordinator"/>) through both adapter kinds. Batches
/// are driven through an explicit posting queue so submission, application and
/// completion stay observable and deterministic without a dispatcher loop.
/// </summary>
public class RustVmBatchTests
{
    private const int InvalidArgument = unchecked((int)0x80070057);

    [Fact]
    public void Generated_adapter_applies_a_valid_batch_and_coalesces_notifications()
    {
        using var host = GeneratedHost();
        var changed = new List<string?>();
        host.Adapter.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        var resets = 0;
        host.Adapter.Items.CollectionChanged += (_, e) =>
        {
            Assert.Equal(NotifyCollectionChangedAction.Reset, e.Action);
            resets++;
        };
        var canExecute = 0;
        host.Adapter.IncrementCommand.CanExecuteChanged += (_, _) => canExecute++;
        var errors = new List<string?>();
        host.Adapter.ErrorsChanged += (_, e) => errors.Add(e.PropertyName);

        // The very first notification must already observe the fully committed
        // batch: fields, command state, errors and collection contents.
        host.Adapter.PropertyChanged += (_, _) =>
        {
            Assert.Equal("Batched", host.Adapter.Name);
            Assert.Equal(7, host.Adapter.Count);
            Assert.Equal(Priority.High, host.Adapter.Priority);
            Assert.Equal(["second", "third", "First"], host.Adapter.Items);
            Assert.False(host.Adapter.IncrementCommand.CanExecute(null));
            Assert.True(host.Adapter.HasErrors);
        };

        var batch = new FakeBatch(1);
        batch.SetString(1, "Batched");
        batch.SetInteger(2, 7);
        batch.SetInteger(6, (long)Priority.High);
        batch.AddString(1, "second");
        batch.AddString(1, "third");
        batch.MoveItem(1, 0, 2);
        batch.SetCommandEnabled(1, 0);
        batch.SetPropertyError(1, "bad");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        Assert.Equal(0, batch.CompletionCount);
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal("Batched", host.Adapter.Name);
        Assert.Equal(7, host.Adapter.Count);
        Assert.Equal(Priority.High, host.Adapter.Priority);
        Assert.Equal(["second", "third", "First"], host.Adapter.Items);
        Assert.False(host.Adapter.IncrementCommand.CanExecute(null));
        Assert.Equal(["bad"], host.Adapter.GetErrors(nameof(host.Adapter.Name)).Cast<string>());

        // One notification per changed member, and exactly one collection Reset
        // for the whole ordered add/add/move sequence.
        Assert.Equal(["Name", "Count", "Priority"], changed);
        Assert.Equal(1, resets);
        Assert.Equal(1, canExecute);
        Assert.Equal(["Name"], errors);
    }

    [Fact]
    public void Table_selection_is_restored_after_snapshot_reset_then_selection_notification()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        using var host = GeneratedHost();
        var sink2 = (IAvnRustVmSink2)host.Model.Sink;
        Assert.Equal(0, sink2.AddModel(3, new FakeNested()));
        Assert.Equal(0, sink2.AddModel(3, new FakeNested()));
        Assert.Equal(0, host.Model.Sink.SetInteger(9, 1));
        Assert.Equal(0, host.Model.Sink.SetString(10, "trace-selected"));
        var table = new TableView
        {
            DataContext = host.Adapter,
            ItemsSource = host.Adapter.TraceRows,
            Template = TableTemplate(),
            ItemContainerTheme = RowTheme(),
            Width = 300,
            Height = 100,
        };
        table.Columns.Add(new TableViewColumn());
        table.Bind(
            SelectingItemsControl.SelectedIndexProperty,
            new Binding(nameof(SampleViewModelAdapter.SelectedTraceIndex)) { Mode = BindingMode.TwoWay });
        var root = new TestRoot(table);
        root.Measure(new Size(300, 100));
        root.Arrange(new Rect(0, 0, 300, 100));
        Assert.Equal(1, table.SelectedIndex);

        var batch = new FakeBatch(1);
        batch.ReplaceModelSnapshot(3, [new FakeNested(), new FakeNested()]);
        batch.SetInteger(9, 0);
        batch.SetString(10, "trace-selected");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();
        root.Measure(new Size(300, 100));
        root.Arrange(new Rect(0, 0, 300, 100));

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(0, host.Adapter.SelectedTraceIndex);
        Assert.Equal("trace-selected", host.Adapter.SelectedTraceKey);
        Assert.Equal(0, table.SelectedIndex);
        // The selection model's Reset write reaches Rust, whose authoritative
        // key/index is synchronously echoed through the sink.
        Assert.Contains((9, -1L), host.Model.IntegerWrites);
    }

    [Fact]
    public void Generated_adapter_rejects_a_batch_whose_later_operation_is_invalid_without_any_change()
    {
        using var host = GeneratedHost();
        var changed = 0;
        host.Adapter.PropertyChanged += (_, _) => changed++;
        var collectionChanges = 0;
        host.Adapter.Items.CollectionChanged += (_, _) => collectionChanges++;

        var batch = new FakeBatch(1);
        batch.SetString(1, "Should not apply");
        batch.AddString(1, "Should not apply either");
        // Valid target, valid wire kind, but the index is past the simulated end.
        batch.ReplaceString(1, 5, "out of range");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal(InvalidArgument, batch.Error);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Equal(["First"], host.Adapter.Items);
        Assert.Equal(0, changed);
        Assert.Equal(0, collectionChanges);
    }

    [Theory]
    [InlineData(99L)] // outside the enum's domain
    [InlineData(-1L)]
    public void Generated_adapter_rejects_an_out_of_domain_enum_value(long value)
    {
        using var host = GeneratedHost();

        var batch = new FakeBatch(1);
        batch.SetString(1, "Should not apply");
        batch.SetInteger(6, value);

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal(InvalidArgument, batch.Error);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Equal(Priority.Normal, host.Adapter.Priority);
    }

    [Fact]
    public void Generated_adapter_rejects_set_null_on_a_non_nullable_property_and_kind_mismatches()
    {
        using var host = GeneratedHost();

        foreach (var invalid in new[]
                 {
                     Build(batch => batch.SetNull(1)),          // non-nullable string
                     Build(batch => batch.SetNull(2)),          // non-nullable integer
                     Build(batch => batch.SetInteger(1, 5)),    // integer on a string property
                     Build(batch => batch.SetString(2, "x")),   // string on an integer property
                     Build(batch => batch.SetBoolean(1, 1)),    // boolean on a string property
                     Build(batch => batch.SetDouble(2, 1.5)),   // double on an integer property
                     Build(batch => batch.SetModel(1, new FakeNested())), // model on a string property
                     Build(batch => batch.SetString(999, "x")), // unknown property
                     Build(batch => batch.SetCommandEnabled(999, 1)), // unknown command
                     Build(batch => batch.SetPropertyError(999, "x")), // unknown property
                     Build(batch => batch.AddString(2, "x")),   // string add on a model collection
                     Build(batch => batch.AddModel(1, new FakeNested())), // model add on a string collection
                     Build(batch => batch.ClearCollection(999)), // unknown collection
                     Build(batch => batch.RemoveAt(1, 1)),      // past the end
                     Build(batch => batch.MoveItem(1, 0, 1)),   // past the end
                 })
        {
            Assert.Equal(0, host.Sink3.SubmitBatch(invalid));
            host.Drain();
            Assert.Equal(RustVmBatchOutcome.Error, invalid.Outcome);
            Assert.Equal(InvalidArgument, invalid.Error);
        }

        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Equal(2, host.Adapter.Count);
        Assert.Equal(["First"], host.Adapter.Items);
        Assert.True(host.Adapter.IncrementCommand.CanExecute(null));
        Assert.False(host.Adapter.HasErrors);

        static FakeBatch Build(Action<FakeBatch> configure)
        {
            // Generation 1 for each: a rejected batch must not advance it.
            var batch = new FakeBatch(1);
            configure(batch);
            return batch;
        }
    }

    [Fact]
    public void Generated_adapter_clears_a_nullable_property_and_a_model_property_through_set_null()
    {
        using var host = GeneratedHost();
        var nestedModel = new FakeNested();

        var first = new FakeBatch(1);
        first.SetString(5, "Nick");
        first.SetModel(7, nestedModel);
        Assert.Equal(0, host.Sink3.SubmitBatch(first));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Applied, first.Outcome);
        Assert.Equal("Nick", host.Adapter.Nickname);
        Assert.NotNull(host.Adapter.Address);
        Assert.Equal(0, nestedModel.DetachCalls);

        var second = new FakeBatch(2);
        second.SetNull(5);
        second.SetNull(7);
        Assert.Equal(0, host.Sink3.SubmitBatch(second));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, second.Outcome);
        Assert.Null(host.Adapter.Nickname);
        Assert.Null(host.Adapter.Address);
        // The displaced nested adapter is detached exactly once, after publication.
        Assert.Equal(1, nestedModel.DetachCalls);
    }

    [Fact]
    public void Generated_adapter_rolls_back_every_staged_adapter_when_a_nested_attach_fails()
    {
        using var host = GeneratedHost();
        var changed = 0;
        host.Adapter.PropertyChanged += (_, _) => changed++;
        var collectionChanges = 0;
        host.Adapter.Tasks.CollectionChanged += (_, _) => collectionChanges++;

        var nestedProperty = new FakeNested();
        var good = new FakeNested();
        var neverStaged = new FakeNested();
        var failing = new FailingNested();

        var batch = new FakeBatch(1);
        batch.SetString(1, "Should not apply");
        batch.SetModel(7, nestedProperty);
        batch.AddModel(2, good);
        batch.AddModel(2, failing);
        batch.AddModel(2, neverStaged);
        // The last add is dropped again, so its model is never attached at all.
        batch.RemoveAt(2, 2);

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Null(host.Adapter.Address);
        Assert.Empty(host.Adapter.Tasks);
        Assert.Equal(0, changed);
        Assert.Equal(0, collectionChanges);

        // Everything staged before the failure was detached again, the failing
        // nested model was detached by its own adapter's constructor, and a
        // model the folded batch dropped was never attached.
        Assert.Equal(1, nestedProperty.DetachCalls);
        Assert.Equal(1, good.DetachCalls);
        Assert.Equal(1, failing.DetachCalls);
        Assert.Equal(0, neverStaged.DetachCalls);
    }

    [Fact]
    public void Generated_adapter_disposes_only_the_nested_adapters_a_batch_displaces()
    {
        using var host = GeneratedHost();

        var kept = new FakeNested();
        var removed = new FakeNested();
        var seed = new FakeBatch(1);
        seed.AddModel(2, kept);
        seed.AddModel(2, removed);
        Assert.Equal(0, host.Sink3.SubmitBatch(seed));
        host.Drain();
        Assert.Equal(2, host.Adapter.Tasks.Count);
        var keptAdapter = host.Adapter.Tasks[0];

        var batch = new FakeBatch(2);
        batch.RemoveAt(2, 1);
        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Single(host.Adapter.Tasks);
        Assert.Same(keptAdapter, host.Adapter.Tasks[0]);
        Assert.Equal(0, kept.DetachCalls);
        Assert.Equal(1, removed.DetachCalls);
    }

    [Fact]
    public void Generated_adapter_replaces_a_hundred_thousand_item_snapshot_with_exactly_one_reset()
    {
        using var host = GeneratedHost();
        var resets = 0;
        var otherActions = 0;
        host.Adapter.Items.CollectionChanged += (_, e) =>
        {
            if (e.Action == NotifyCollectionChangedAction.Reset)
                resets++;
            else
                otherActions++;
        };

        var values = Enumerable.Range(0, 100_000).Select(index => $"item-{index}").ToArray();
        var batch = new FakeBatch(1);
        batch.ReplaceStringSnapshot(1, values);

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(100_000, host.Adapter.Items.Count);
        Assert.Equal("item-0", host.Adapter.Items[0]);
        Assert.Equal("item-99999", host.Adapter.Items[99_999]);
        Assert.Equal(1, resets);
        Assert.Equal(0, otherActions);
    }

    [Fact]
    public void Generated_adapter_reports_equal_and_lower_generations_as_stale()
    {
        using var host = GeneratedHost();

        var first = new FakeBatch(5);
        first.SetString(1, "Fifth");
        Assert.Equal(0, host.Sink3.SubmitBatch(first));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Applied, first.Outcome);

        foreach (var generation in new long[] { 5, 4, 0 })
        {
            var stale = new FakeBatch(generation);
            stale.SetString(1, "Stale");
            Assert.Equal(0, host.Sink3.SubmitBatch(stale));
            host.Drain();
            Assert.Equal(RustVmBatchOutcome.Stale, stale.Outcome);
            Assert.Equal("Fifth", host.Adapter.Name);
        }

        var newer = new FakeBatch(6);
        newer.SetString(1, "Sixth");
        Assert.Equal(0, host.Sink3.SubmitBatch(newer));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Applied, newer.Outcome);
        Assert.Equal("Sixth", host.Adapter.Name);
    }

    [Fact]
    public void Generated_adapter_rejects_a_batch_whose_getter_fails_without_touching_state()
    {
        using var host = GeneratedHost();

        var batch = new FakeBatch(1);
        batch.SetString(1, "Should not apply");
        batch.SetInteger(2, 42);
        batch.FailGetterAt = 1;

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal(unchecked((int)0x80004005), batch.Error);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Equal(2, host.Adapter.Count);
    }

    [Fact]
    public async Task Submission_never_completes_or_applies_on_the_submitting_stack()
    {
        using var host = GeneratedHost();
        var batch = new FakeBatch(1);
        batch.SetString(1, "Worker");

        var result = await Task.Run(() => host.Sink3.SubmitBatch(batch));

        Assert.Equal(0, result);
        Assert.Equal(0, batch.CompletionCount);
        Assert.Empty(batch.CompletionStacks);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Single(host.Pending);

        host.Drain();
        Assert.Equal(1, batch.CompletionCount);
        Assert.Equal("Worker", host.Adapter.Name);
        // The completion ran from the posted work item, not SubmitBatch.
        Assert.DoesNotContain(nameof(IAvnRustVmSink3.SubmitBatch), batch.CompletionStacks[0], StringComparison.Ordinal);
    }

    [Fact]
    public void Batches_queued_before_disposal_are_cancelled_and_later_submissions_are_cancelled_too()
    {
        var host = GeneratedHost();
        var queued = new FakeBatch(1);
        queued.SetString(1, "Never");
        Assert.Equal(0, host.Sink3.SubmitBatch(queued));

        host.Adapter.Dispose();
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Cancelled, queued.Outcome);
        Assert.Equal("Initial", host.Adapter.Name);

        var afterDispose = new FakeBatch(2);
        afterDispose.SetString(1, "Also never");
        Assert.Equal(0, host.Sink3.SubmitBatch(afterDispose));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Cancelled, afterDispose.Outcome);
        Assert.Equal("Initial", host.Adapter.Name);
    }

    [Fact]
    public void Reentrant_dispose_from_a_batch_notification_defers_cleanup_until_the_batch_finishes()
    {
        var host = GeneratedHost();
        var nested = new FakeNested();
        var seed = new FakeBatch(1);
        seed.AddModel(2, nested);
        Assert.Equal(0, host.Sink3.SubmitBatch(seed));
        host.Drain();
        Assert.Single(host.Adapter.Tasks);

        var observed = new List<string?>();
        var resetsAfterDispose = 0;
        host.Adapter.Items.CollectionChanged += (_, _) => resetsAfterDispose++;
        host.Adapter.PropertyChanged += (_, e) =>
        {
            observed.Add(e.PropertyName);
            if (e.PropertyName == nameof(host.Adapter.Name))
            {
                // Reentrant disposal must not detach the model or tear nested
                // adapters down while the batch is still publishing.
                host.Adapter.Dispose();
                Assert.Equal(0, host.Model.DetachCalls);
                Assert.Equal(0, nested.DetachCalls);
            }
        };

        var batch = new FakeBatch(2);
        batch.SetString(1, "Reentrant");
        batch.SetInteger(2, 11);
        batch.AddString(1, "still published");
        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(["Name", "Count"], observed);
        Assert.Equal(1, resetsAfterDispose);
        Assert.Equal("Reentrant", host.Adapter.Name);
        // Deferred cleanup ran exactly once, after the batch left the gate.
        Assert.Equal(1, host.Model.DetachCalls);
        Assert.Equal(1, nested.DetachCalls);

        host.Adapter.Dispose();
        Assert.Equal(1, host.Model.DetachCalls);
    }

    [Fact]
    public void A_throwing_observer_does_not_undo_a_committed_batch_or_report_a_rollback()
    {
        using var host = GeneratedHost();
        host.Adapter.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(host.Adapter.Name))
                throw new InvalidOperationException("observer");
        };
        var sawCount = false;
        host.Adapter.PropertyChanged += (_, e) => sawCount |= e.PropertyName == nameof(host.Adapter.Count);

        var batch = new FakeBatch(1);
        batch.SetString(1, "Committed");
        batch.SetInteger(2, 3);

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(0, batch.Error);
        Assert.Equal("Committed", host.Adapter.Name);
        Assert.Equal(3, host.Adapter.Count);
        Assert.True(sawCount, "a failing observer must not suppress later notifications");
    }

    [Fact]
    public async Task Concurrent_dispose_detaches_exactly_once()
    {
        var host = GeneratedHost();
        using var start = new ManualResetEventSlim();

        var disposals = Enumerable.Range(0, 8).Select(_ => Task.Run(() =>
        {
            start.Wait();
            host.Adapter.Dispose();
        })).ToArray();

        start.Set();
        await Task.WhenAll(disposals);

        Assert.Equal(1, host.Model.DetachCalls);
    }

    [Fact]
    public void Reflectable_adapter_applies_collection_operations_including_clear()
    {
        using var host = ReflectableHost();
        var items = Items(host.Adapter, "Items");
        var tasks = Items(host.Adapter, "Tasks");
        var itemResets = 0;
        var taskResets = 0;
        items.CollectionChanged += (_, e) =>
        {
            Assert.Equal(NotifyCollectionChangedAction.Reset, e.Action);
            itemResets++;
        };
        tasks.CollectionChanged += (_, e) =>
        {
            Assert.Equal(NotifyCollectionChangedAction.Reset, e.Action);
            taskResets++;
        };

        var nested = new FakeNested();
        var seed = new FakeBatch(1);
        seed.AddString(1, "second");
        seed.AddString(1, "third");
        seed.AddModel(2, nested);
        Assert.Equal(0, host.Sink3.SubmitBatch(seed));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Applied, seed.Outcome);
        Assert.Equal(new object?[] { "First", "second", "third" }, items);
        Assert.Single(tasks);

        // ClearCollection (kind 19) is a first-class batch operation for both
        // string and nested-model collections.
        var clear = new FakeBatch(2);
        clear.ClearCollection(1);
        clear.ClearCollection(2);
        Assert.Equal(0, host.Sink3.SubmitBatch(clear));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, clear.Outcome);
        Assert.Empty(items);
        Assert.Empty(tasks);
        Assert.Equal(1, nested.DetachCalls);
        Assert.Equal(2, itemResets);
        Assert.Equal(2, taskResets);
    }

    [Fact]
    public void Reflectable_adapter_rejects_invalid_enum_null_and_index_operations_without_mutating()
    {
        using var host = ReflectableHost();
        var items = Items(host.Adapter, "Items");
        var changed = 0;
        ((INotifyPropertyChanged)host.Adapter).PropertyChanged += (_, _) => changed++;

        var generation = 1L;
        foreach (var configure in new Action<FakeBatch>[]
                 {
                     batch => batch.SetInteger(6, 99),
                     batch => batch.SetNull(1),
                     batch => batch.SetInteger(1, 4),
                     batch => batch.InsertString(1, 4, "x"),
                     batch => batch.ReplaceModel(2, 0, new FakeNested()),
                     batch => batch.ClearCollection(77),
                 })
        {
            var batch = new FakeBatch(generation++);
            batch.SetString(1, "Should not apply");
            configure(batch);
            Assert.Equal(0, host.Sink3.SubmitBatch(batch));
            host.Drain();
            Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
            Assert.Equal(InvalidArgument, batch.Error);
        }

        Assert.Equal("Initial", Member(host.Adapter, "Name"));
        Assert.Equal(new object?[] { "First" }, items);
        Assert.Equal(0, changed);
    }

    [Fact]
    public void Reflectable_adapter_rolls_back_staged_adapters_when_a_nested_attach_fails()
    {
        using var host = ReflectableHost();
        var tasks = Items(host.Adapter, "Tasks");
        var good = new FakeNested();

        var batch = new FakeBatch(1);
        batch.SetString(1, "Should not apply");
        batch.AddModel(2, good);
        batch.AddModel(2, new FailingNested());

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal("Initial", Member(host.Adapter, "Name"));
        Assert.Empty(tasks);
        Assert.Equal(1, good.DetachCalls);
    }

    [Fact]
    public void Reflectable_adapter_replaces_a_hundred_thousand_item_snapshot_with_exactly_one_reset()
    {
        using var host = ReflectableHost();
        var items = Items(host.Adapter, "Items");
        var resets = 0;
        items.CollectionChanged += (_, e) =>
        {
            Assert.Equal(NotifyCollectionChangedAction.Reset, e.Action);
            resets++;
        };

        var batch = new FakeBatch(1);
        batch.ReplaceStringSnapshot(1, Enumerable.Range(0, 100_000).Select(index => $"item-{index}").ToArray());

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(100_000, items.Count);
        Assert.Equal("item-99999", items[99_999]);
        Assert.Equal(1, resets);
    }

    [Fact]
    public void Reflectable_adapter_defers_reentrant_dispose_and_cancels_queued_batches()
    {
        var host = ReflectableHost();
        var nested = new FakeNested();
        var seed = new FakeBatch(1);
        seed.AddModel(2, nested);
        Assert.Equal(0, host.Sink3.SubmitBatch(seed));
        host.Drain();

        ((INotifyPropertyChanged)host.Adapter).PropertyChanged += (_, e) =>
        {
            if (e.PropertyName != "Name")
                return;
            host.Adapter.Dispose();
            Assert.Equal(0, host.Model.DetachCalls);
            Assert.Equal(0, nested.DetachCalls);
        };

        var batch = new FakeBatch(2);
        batch.SetString(1, "Reentrant");
        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal("Reentrant", Member(host.Adapter, "Name"));
        Assert.Equal(1, host.Model.DetachCalls);
        Assert.Equal(1, nested.DetachCalls);

        var queued = new FakeBatch(3);
        queued.SetString(1, "Never");
        Assert.Equal(0, host.Sink3.SubmitBatch(queued));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Cancelled, queued.Outcome);
    }

    [Fact]
    public void A_target_whose_commit_throws_reports_an_error_and_releases_every_staged_adapter()
    {
        var pending = new Queue<Action>();
        var target = new ThrowingCommitTarget();
        var coordinator = new RustVmBatchCoordinator(target, pending.Enqueue);

        var batch = new FakeBatch(1);
        batch.SetString(1, "throws on commit");
        batch.AddModel(2, new FakeNested());
        batch.AddModel(2, new FakeNested());

        Assert.Equal(0, coordinator.Submit(batch));
        while (pending.Count > 0)
            pending.Dequeue()();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        // Staging succeeded, so both elements exist; the interrupted commit must
        // still release them rather than strand attached nested models.
        Assert.Equal(2, target.Created.Count);
        Assert.All(target.Created, staged => Assert.True(staged.Disposed));
    }

    private sealed class ThrowingCommitTarget : IRustVmBatchTarget
    {
        public List<StagedElement> Created { get; } = [];

        public bool TryGetProperty(int propertyId, out RustVmBatchProperty property)
        {
            property = propertyId == 1
                ? new RustVmBatchProperty("Name", RustVmValueWireKind.String, false, false)
                : default;
            return property.Name is not null;
        }

        public bool TryGetCollection(int collectionId, out RustVmBatchCollectionInfo collection)
        {
            collection = collectionId == 2
                ? new RustVmBatchCollectionInfo("Tasks", RustVmValueWireKind.Model, Items)
                : default;
            return collection.Items is not null;
        }

        public BatchObservableCollection<object?> Items { get; } = [];

        public bool TryGetCommand(int commandId, out IRustVmBatchCommand command)
        {
            command = null!;
            return false;
        }

        public bool IsEnumValueDefined(int propertyId, long value) => false;

        public IDisposable CreateNestedProperty(int propertyId, IAvnRustViewModel model) =>
            throw new NotSupportedException();

        public IDisposable CreateNestedElement(int collectionId, IAvnRustViewModel model)
        {
            var element = new StagedElement();
            Created.Add(element);
            return element;
        }

        public bool CommitProperty(int propertyId, in RustVmBatchValue value, out IDisposable? replaced) =>
            throw new InvalidOperationException("commit failed");

        public bool CommitError(string propertyName, string? message) => false;
        public void RaisePropertyChanged(string propertyName) { }
        public void RaiseErrorsChanged(string propertyName) { }
    }

    private sealed class StagedElement : IDisposable
    {
        public bool Disposed { get; private set; }

        public void Dispose() => Disposed = true;
    }

    [Fact]
    public void Ownership_commits_once_between_the_state_commit_and_the_notifications()
    {
        using var host = GeneratedHost();
        var batch = new FakeBatch(1);
        batch.SetString(1, "Batched");
        batch.SetInteger(2, 5);
        batch.AddString(1, "second");
        batch.SetCommandEnabled(1, 0);
        batch.SetPropertyError(1, "bad");

        host.Adapter.PropertyChanged += (_, e) =>
        {
            // Every notification-free store already happened, and so did the
            // producer's ownership transfer.
            Assert.Equal(1, batch.OwnershipCommits);
            batch.Trace.Add($"property:{e.PropertyName}");
        };
        host.Adapter.Items.CollectionChanged += (_, _) => batch.Trace.Add("collection");
        host.Adapter.IncrementCommand.CanExecuteChanged += (_, _) => batch.Trace.Add("command");
        host.Adapter.ErrorsChanged += (_, e) => batch.Trace.Add($"errors:{e.PropertyName}");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(1, batch.OwnershipCommits);
        Assert.Equal(
            [
                "ownership",
                "property:Name",
                "property:Count",
                "collection",
                "command",
                "errors:Name",
                "complete:Applied",
            ],
            batch.Trace);
    }

    [Fact]
    public void Ownership_is_not_committed_for_stale_cancelled_or_rejected_batches()
    {
        var host = GeneratedHost();

        var applied = new FakeBatch(5);
        applied.SetString(1, "Fifth");
        Assert.Equal(0, host.Sink3.SubmitBatch(applied));
        host.Drain();
        Assert.Equal(1, applied.OwnershipCommits);

        var stale = new FakeBatch(5);
        stale.SetString(1, "Stale");
        Assert.Equal(0, host.Sink3.SubmitBatch(stale));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Stale, stale.Outcome);
        Assert.Equal(0, stale.OwnershipCommits);

        var invalid = new FakeBatch(6);
        invalid.SetString(1, "Invalid");
        invalid.ReplaceString(1, 9, "out of range");
        Assert.Equal(0, host.Sink3.SubmitBatch(invalid));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Error, invalid.Outcome);
        Assert.Equal(0, invalid.OwnershipCommits);

        var failedStaging = new FakeBatch(7);
        failedStaging.AddModel(2, new FailingNested());
        Assert.Equal(0, host.Sink3.SubmitBatch(failedStaging));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Error, failedStaging.Outcome);
        Assert.Equal(0, failedStaging.OwnershipCommits);

        var cancelled = new FakeBatch(8);
        cancelled.SetString(1, "Never");
        Assert.Equal(0, host.Sink3.SubmitBatch(cancelled));
        host.Adapter.Dispose();
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Cancelled, cancelled.Outcome);
        Assert.Equal(0, cancelled.OwnershipCommits);

        Assert.Equal("Fifth", host.Adapter.Name);
    }

    [Fact]
    public void A_batch_without_the_ownership_capability_is_rejected_with_zero_mutation()
    {
        using var host = GeneratedHost();
        var changed = 0;
        host.Adapter.PropertyChanged += (_, _) => changed++;
        var collectionChanges = 0;
        host.Adapter.Items.CollectionChanged += (_, _) => collectionChanges++;

        var batch = new LegacyBatch(1);
        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Error, batch.Outcome);
        Assert.Equal(unchecked((int)0x80004002), batch.Error);
        Assert.Equal("Initial", host.Adapter.Name);
        Assert.Equal(["First"], host.Adapter.Items);
        Assert.Equal(0, changed);
        Assert.Equal(0, collectionChanges);
        // A rejected batch does not advance the generation.
        Assert.Equal(0, batch.OperationReads);

        var next = new FakeBatch(1);
        next.SetString(1, "Applies");
        Assert.Equal(0, host.Sink3.SubmitBatch(next));
        host.Drain();
        Assert.Equal(RustVmBatchOutcome.Applied, next.Outcome);
        Assert.Equal("Applies", host.Adapter.Name);
    }

    [Fact]
    public void A_nested_update_published_from_a_notification_lands_after_ownership_transfer()
    {
        using var host = GeneratedHost();
        var batch = new FakeBatch(1);
        batch.SetString(1, "Batched");
        batch.AddModel(2, new FakeNested());

        var sink2 = (IAvnRustVmSink2)host.Model.Sink;
        var reentrant = new FakeNested();
        var published = false;
        host.Adapter.PropertyChanged += (_, _) =>
        {
            if (published)
                return;
            published = true;
            // A reentrant synchronous nested publish from an observer: the
            // batch's own ownership must already be committed.
            Assert.Equal(1, batch.OwnershipCommits);
            Assert.Equal(0, sink2.AddModel(2, reentrant));
            batch.Trace.Add("reentrant-add");
        };

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(2, host.Adapter.Tasks.Count);
        Assert.Equal(
            ["ownership", "reentrant-add", "complete:Applied"],
            batch.Trace.Where(entry => entry is "ownership" or "reentrant-add" or "complete:Applied"));
    }

    [Fact]
    public void A_failed_ownership_commit_is_logged_without_undoing_the_committed_batch()
    {
        using var host = GeneratedHost();
        var batch = new FakeBatch(1) { OwnershipResult = unchecked((int)0x80004005) };
        batch.SetString(1, "Committed");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(1, batch.OwnershipCommits);
        Assert.Equal("Committed", host.Adapter.Name);
    }

    [Fact]
    public void Reflectable_adapter_commits_ownership_before_notifying()
    {
        using var host = ReflectableHost();
        var batch = new FakeBatch(1);
        batch.SetString(1, "Batched");
        batch.AddString(1, "second");

        ((INotifyPropertyChanged)host.Adapter).PropertyChanged += (_, e) =>
        {
            Assert.Equal(1, batch.OwnershipCommits);
            batch.Trace.Add($"property:{e.PropertyName}");
        };
        Items(host.Adapter, "Items").CollectionChanged += (_, _) => batch.Trace.Add("collection");

        Assert.Equal(0, host.Sink3.SubmitBatch(batch));
        host.Drain();

        Assert.Equal(RustVmBatchOutcome.Applied, batch.Outcome);
        Assert.Equal(
            ["ownership", "property:Name", "collection", "complete:Applied"],
            batch.Trace);
    }

    [Fact]
    public void Reflectable_adapter_preserves_its_original_constructor_signatures()
    {
        // Optional parameters do not preserve CLR constructor signatures, so the
        // pre-batch arities must remain distinct overloads for already-compiled
        // callers.
        var type = typeof(ReflectableRustViewModelAdapter);
        Assert.NotNull(type.GetConstructor(
            [typeof(IAvnRustViewModel), typeof(RustViewModelDescriptor)]));
        Assert.NotNull(type.GetConstructor(
            [typeof(IAvnRustViewModel), typeof(RustViewModelDescriptor), typeof(Action<Action>)]));
        Assert.NotNull(type.GetConstructor(
            [
                typeof(IAvnRustViewModel),
                typeof(RustViewModelDescriptor),
                typeof(Action<Action>),
                typeof(Action<Action>),
            ]));

        var generated = typeof(SampleViewModelAdapter);
        Assert.NotNull(generated.GetConstructor([typeof(IAvnRustViewModel)]));
        Assert.NotNull(generated.GetConstructor([typeof(IAvnRustViewModel), typeof(Action<Action>)]));
        Assert.NotNull(generated.GetConstructor(
            [typeof(IAvnRustViewModel), typeof(Action<Action>), typeof(Action<Action>)]));

        // Late-bound activation through the historical 3-argument shape still works.
        var model = new BatchModel();
        using var adapter = (ReflectableRustViewModelAdapter)Activator.CreateInstance(
            type,
            [model, SampleViewModelMetadata.Descriptor, (Action<Action>)(action => action())])!;
        Assert.Equal("Initial", Member(adapter, "Name"));
    }

    private static FuncControlTemplate TableTemplate() =>
        new FuncControlTemplate<TableView>((parent, scope) =>
            new DockPanel
            {
                Children =
                {
                    new TableViewColumnHeadersPresenter { [DockPanel.DockProperty] = Dock.Top },
                    new ScrollViewer
                    {
                        Name = "PART_ScrollViewer",
                        Template = new FuncControlTemplate<ScrollViewer>((_, scope) =>
                            new Panel
                            {
                                Children =
                                {
                                    new ScrollContentPresenter { Name = "PART_ContentPresenter" }
                                        .RegisterInNameScope(scope),
                                },
                            }),
                        Content = new ItemsPresenter
                        {
                            Name = "PART_ItemsPresenter",
                            [~ItemsPresenter.ItemsPanelProperty] =
                                parent.GetObservable(ItemsControl.ItemsPanelProperty).ToBinding(),
                        }.RegisterInNameScope(scope),
                    }.RegisterInNameScope(scope),
                },
            });

    private static ControlTheme RowTheme() => new(typeof(TableViewRow))
    {
        Setters =
        {
            new Setter(
                TemplatedControl.TemplateProperty,
                new FuncControlTemplate<TableViewRow>((_, scope) =>
                    new TableViewCellsPresenter { Name = "PART_CellsPresenter" }
                        .RegisterInNameScope(scope))),
        },
    };

    private static GeneratedFixture GeneratedHost()
    {
        var model = new BatchModel();
        var pending = new Queue<Action>();
        var adapter = new SampleViewModelAdapter(model, action => action(), pending.Enqueue);
        return new GeneratedFixture(model, adapter, pending);
    }

    private static ReflectableFixture ReflectableHost()
    {
        var model = new BatchModel();
        var pending = new Queue<Action>();
        var adapter = new ReflectableRustViewModelAdapter(
            model, SampleViewModelMetadata.Descriptor, action => action(), pending.Enqueue);
        return new ReflectableFixture(model, adapter, pending);
    }

    private static ObservableCollection<object?> Items(ReflectableRustViewModelAdapter adapter, string name) =>
        (ObservableCollection<object?>)Member(adapter, name)!;

    private static object? Member(ReflectableRustViewModelAdapter adapter, string name) =>
        ((IReflectableType)adapter).GetTypeInfo().GetProperty(name)!.GetValue(adapter);

    private sealed class GeneratedFixture(BatchModel model, SampleViewModelAdapter adapter, Queue<Action> pending)
        : IDisposable
    {
        public BatchModel Model { get; } = model;
        public SampleViewModelAdapter Adapter { get; } = adapter;
        public Queue<Action> Pending { get; } = pending;
        public IAvnRustVmSink3 Sink3 => (IAvnRustVmSink3)Model.Sink;

        public void Drain()
        {
            while (Pending.Count > 0)
                Pending.Dequeue()();
        }

        public void Dispose() => Adapter.Dispose();
    }

    private sealed class ReflectableFixture(
        BatchModel model,
        ReflectableRustViewModelAdapter adapter,
        Queue<Action> pending) : IDisposable
    {
        public BatchModel Model { get; } = model;
        public ReflectableRustViewModelAdapter Adapter { get; } = adapter;
        public Queue<Action> Pending { get; } = pending;
        public IAvnRustVmSink3 Sink3 => (IAvnRustVmSink3)Model.Sink;

        public void Drain()
        {
            while (Pending.Count > 0)
                Pending.Dequeue()();
        }

        public void Dispose() => Adapter.Dispose();
    }

    /// <summary>
    /// A minimal model matching <c>SampleViewModel</c>'s seed state. Batch tests
    /// publish through the sink rather than through commands.
    /// </summary>
    private sealed class BatchModel : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;

        public IAvnRustVmSink Sink => _sink!;
        public int DetachCalls { get; private set; }
        public List<(int PropertyId, long Value)> IntegerWrites { get; } = [];

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = sink;
            sink!.SetString(1, "Initial");
            sink.SetInteger(2, 2);
            sink.AddString(1, "First");
            return 0;
        }

        public int Detach()
        {
            DetachCalls++;
            return 0;
        }

        public int SetString(int propertyId, string? value) => 0;
        public int SetInteger(int propertyId, long value)
        {
            IntegerWrites.Add((propertyId, value));
            // Models the Rust table owner retaining its declared row key during
            // a Reset and rejecting the selection model's transient -1 write.
            if (propertyId == 9 && value < 0)
                _sink!.SetInteger(9, 0);
            return 0;
        }
        public int SetBoolean(int propertyId, int value) => 0;
        public int SetDouble(int propertyId, double value) => 0;
        public int Execute(int commandId, string? parameter) => 0;
        public int BeginAsync(int commandId, string? parameter) => 0;
    }

    private sealed class FakeNested : IAvnRustViewModel
    {
        public int DetachCalls { get; private set; }

        public int Attach(IAvnRustVmSink? sink) => 0;
        public int Detach() { DetachCalls++; return 0; }
        public int SetString(int propertyId, string? value) => 0;
        public int SetInteger(int propertyId, long value) => 0;
        public int SetBoolean(int propertyId, int value) => 0;
        public int SetDouble(int propertyId, double value) => 0;
        public int Execute(int commandId, string? parameter) => 0;
        public int BeginAsync(int commandId, string? parameter) => 0;
    }

    private sealed class FailingNested : IAvnRustViewModel
    {
        public int DetachCalls { get; private set; }

        public int Attach(IAvnRustVmSink? sink) => unchecked((int)0x80004005);
        public int Detach() { DetachCalls++; return 0; }
        public int SetString(int propertyId, string? value) => 0;
        public int SetInteger(int propertyId, long value) => 0;
        public int SetBoolean(int propertyId, int value) => 0;
        public int SetDouble(int propertyId, double value) => 0;
        public int Execute(int commandId, string? parameter) => 0;
        public int BeginAsync(int commandId, string? parameter) => 0;
    }

    /// <summary>
    /// A managed stand-in for Rust's immutable nano-COM batch. It records
    /// completion (count, outcome, error and the completing call stack) so tests
    /// can assert that nothing completes on the submitting stack.
    /// </summary>
    private sealed class FakeBatch(long generation) : IAvnRustVmUpdateBatch, IAvnRustVmUpdateBatch2
    {
        private readonly List<Operation> _operations = [];

        public RustVmBatchOutcome Outcome { get; private set; }
        public int Error { get; private set; }
        public int CompletionCount { get; private set; }
        public List<string> CompletionStacks { get; } = [];

        /// <summary>Ordered trace of ownership commits and completions.</summary>
        public List<string> Trace { get; } = [];

        public int OwnershipCommits { get; private set; }

        /// <summary>Makes the operation at this index fail one of its getters.</summary>
        public int FailGetterAt { get; set; } = -1;

        /// <summary>Forces <see cref="CommitOwnership"/> to report a failure.</summary>
        public int OwnershipResult { get; set; }

        public void SetString(int propertyId, string value) => Add(RustVmUpdateKind.SetString, propertyId, text: value);
        public void SetInteger(int propertyId, long value) => Add(RustVmUpdateKind.SetInteger, propertyId, integer: value);
        public void SetBoolean(int propertyId, int value) => Add(RustVmUpdateKind.SetBoolean, propertyId, boolean: value);
        public void SetDouble(int propertyId, double value) => Add(RustVmUpdateKind.SetDouble, propertyId, number: value);
        public void SetNull(int propertyId) => Add(RustVmUpdateKind.SetNull, propertyId);
        public void SetModel(int propertyId, IAvnRustViewModel model) => Add(RustVmUpdateKind.SetModel, propertyId, model: model);
        public void AddString(int collectionId, string value) => Add(RustVmUpdateKind.AddString, collectionId, text: value);
        public void AddModel(int collectionId, IAvnRustViewModel model) => Add(RustVmUpdateKind.AddModel, collectionId, model: model);
        public void InsertString(int collectionId, int index, string value) => Add(RustVmUpdateKind.InsertString, collectionId, index: index, text: value);
        public void ReplaceString(int collectionId, int index, string value) => Add(RustVmUpdateKind.ReplaceString, collectionId, index: index, text: value);
        public void ReplaceModel(int collectionId, int index, IAvnRustViewModel model) => Add(RustVmUpdateKind.ReplaceModel, collectionId, index: index, model: model);
        public void RemoveAt(int collectionId, int index) => Add(RustVmUpdateKind.RemoveAt, collectionId, index: index);
        public void MoveItem(int collectionId, int from, int to) => Add(RustVmUpdateKind.MoveItem, collectionId, index: from, index2: to);
        public void ClearCollection(int collectionId) => Add(RustVmUpdateKind.ClearCollection, collectionId);
        public void SetCommandEnabled(int commandId, int enabled) => Add(RustVmUpdateKind.SetCommandEnabled, commandId, boolean: enabled);
        public void SetPropertyError(int propertyId, string message) => Add(RustVmUpdateKind.SetPropertyError, propertyId, text: message);
        public void ClearPropertyError(int propertyId) => Add(RustVmUpdateKind.SetPropertyError, propertyId, boolean: 1);
        public void ReplaceStringSnapshot(int collectionId, IReadOnlyList<string> values) =>
            Add(RustVmUpdateKind.ReplaceStringSnapshot, collectionId, strings: values);
        public void ReplaceModelSnapshot(int collectionId, IReadOnlyList<IAvnRustViewModel> values) =>
            Add(RustVmUpdateKind.ReplaceModelSnapshot, collectionId, models: values);

        public int GetGeneration(out long value)
        {
            value = generation;
            return 0;
        }

        public int GetOperationCount(out int count)
        {
            count = _operations.Count;
            return 0;
        }

        public int GetOperation(int index, out IAvnRustVmUpdateOperation? operation)
        {
            if ((uint)index >= (uint)_operations.Count)
            {
                operation = null;
                return InvalidArgument;
            }
            operation = new FakeOperation(_operations[index], index == FailGetterAt);
            return 0;
        }

        public int GetSnapshotItemCount(int operationIndex, out int count)
        {
            count = -1;
            if ((uint)operationIndex >= (uint)_operations.Count)
                return InvalidArgument;
            var operation = _operations[operationIndex];
            count = operation.Strings?.Count ?? operation.Models?.Count ?? -1;
            return count < 0 ? InvalidArgument : 0;
        }

        public int GetSnapshotStringLength(int operationIndex, int itemIndex, out int length)
        {
            length = 0;
            var values = _operations[operationIndex].Strings;
            if (values is null || (uint)itemIndex >= (uint)values.Count)
                return InvalidArgument;
            length = values[itemIndex].Length;
            return 0;
        }

        public unsafe int CopySnapshotString(int operationIndex, int itemIndex, char* destination, int capacity)
        {
            var values = _operations[operationIndex].Strings;
            if (values is null || (uint)itemIndex >= (uint)values.Count)
                return InvalidArgument;
            return Copy(values[itemIndex], destination, capacity);
        }

        public int GetSnapshotModel(int operationIndex, int itemIndex, out IAvnRustViewModel? model)
        {
            model = null;
            var values = _operations[operationIndex].Models;
            if (values is null || (uint)itemIndex >= (uint)values.Count)
                return InvalidArgument;
            model = values[itemIndex];
            return 0;
        }

        public int Complete(int outcome, int error)
        {
            CompletionCount++;
            Outcome = (RustVmBatchOutcome)outcome;
            Error = error;
            CompletionStacks.Add(Environment.StackTrace);
            Trace.Add($"complete:{(RustVmBatchOutcome)outcome}");
            return 0;
        }

        public int CommitOwnership()
        {
            OwnershipCommits++;
            Trace.Add("ownership");
            return OwnershipResult;
        }

        internal static unsafe int Copy(string value, char* destination, int capacity)
        {
            if (destination is null || capacity < value.Length + 1)
                return InvalidArgument;
            for (var index = 0; index < value.Length; index++)
                destination[index] = value[index];
            destination[value.Length] = '\0';
            return 0;
        }

        private void Add(
            RustVmUpdateKind kind,
            int target,
            int index = 0,
            int index2 = 0,
            long integer = 0,
            double number = 0,
            int boolean = 0,
            string? text = null,
            IAvnRustViewModel? model = null,
            IReadOnlyList<string>? strings = null,
            IReadOnlyList<IAvnRustViewModel>? models = null) =>
            _operations.Add(new Operation(kind, target, index, index2, integer, number, boolean, text, model, strings, models));

        internal sealed record Operation(
            RustVmUpdateKind Kind,
            int Target,
            int Index,
            int Index2,
            long Integer,
            double Double,
            int Boolean,
            string? Text,
            IAvnRustViewModel? Model,
            IReadOnlyList<string>? Strings,
            IReadOnlyList<IAvnRustViewModel>? Models);
    }

    /// <summary>
    /// A batch that implements only the original immutable-batch contract. It
    /// stands in for a producer that has not been rebuilt against the
    /// ownership-commit capability.
    /// </summary>
    private sealed class LegacyBatch(long generation) : IAvnRustVmUpdateBatch
    {
        public RustVmBatchOutcome Outcome { get; private set; }
        public int Error { get; private set; }

        /// <summary>Counts reads the coordinator must never perform.</summary>
        public int OperationReads { get; private set; }

        public int GetGeneration(out long value)
        {
            value = generation;
            return 0;
        }

        public int GetOperationCount(out int count)
        {
            OperationReads++;
            count = 0;
            return 0;
        }

        public int GetOperation(int index, out IAvnRustVmUpdateOperation? operation)
        {
            OperationReads++;
            operation = null;
            return InvalidArgument;
        }

        public int GetSnapshotItemCount(int operationIndex, out int count)
        {
            OperationReads++;
            count = 0;
            return InvalidArgument;
        }

        public int GetSnapshotStringLength(int operationIndex, int itemIndex, out int length)
        {
            OperationReads++;
            length = 0;
            return InvalidArgument;
        }

        public unsafe int CopySnapshotString(int operationIndex, int itemIndex, char* destination, int capacity)
        {
            OperationReads++;
            return InvalidArgument;
        }

        public int GetSnapshotModel(int operationIndex, int itemIndex, out IAvnRustViewModel? model)
        {
            OperationReads++;
            model = null;
            return InvalidArgument;
        }

        public int Complete(int outcome, int error)
        {
            Outcome = (RustVmBatchOutcome)outcome;
            Error = error;
            return 0;
        }
    }

    private sealed class FakeOperation(FakeBatch.Operation operation, bool fail) : IAvnRustVmUpdateOperation
    {
        public int GetKind(out int kind)
        {
            kind = (int)operation.Kind;
            return 0;
        }

        public int GetTargetId(out int targetId)
        {
            targetId = operation.Target;
            return 0;
        }

        public int GetIndex(out int index)
        {
            index = operation.Index;
            return 0;
        }

        public int GetIndex2(out int index)
        {
            index = operation.Index2;
            return 0;
        }

        public int GetInteger(out long value)
        {
            value = operation.Integer;
            // The engine must check every getter, not just the first few.
            return fail ? unchecked((int)0x80004005) : 0;
        }

        public int GetDouble(out double value)
        {
            value = operation.Double;
            return 0;
        }

        public int GetBoolean(out int value)
        {
            value = operation.Boolean;
            return 0;
        }

        public int GetTextLength(out int length)
        {
            length = operation.Text?.Length ?? 0;
            return 0;
        }

        public unsafe int CopyText(char* destination, int capacity) =>
            FakeBatch.Copy(operation.Text ?? string.Empty, destination, capacity);

        public int GetModel(out IAvnRustViewModel? model)
        {
            model = operation.Model;
            return 0;
        }
    }
}
