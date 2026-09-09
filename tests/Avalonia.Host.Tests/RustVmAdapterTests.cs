using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Rust;
using Avalonia.Rust.Interop;
using Avalonia.Rust.Sample.Generated;
using Avalonia.Rust.Sample.Views;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class RustVmAdapterTests
{
    [Fact]
    public void Adapter_forwards_presentation_commands_to_model_and_sink()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());

        Assert.Equal("Initial", adapter.Name);
        Assert.Equal(2, adapter.Count);
        Assert.Equal(["First"], adapter.Items);

        adapter.Name = "Edited";
        adapter.IncrementCommand.Execute(null);
        adapter.NewItem = "Second";
        adapter.AddCommand.Execute(null);
        adapter.SaveCommand.Execute(null);

        Assert.Equal("Edited", model.Name);
        Assert.Equal(1, model.IncrementCalls);
        Assert.Equal(["Second"], model.AddedItems);
        Assert.Equal(1, model.SaveCalls);
        Assert.Equal("", adapter.NewItem);

        var sink = model.Sink;
        adapter.Dispose();
        Assert.Equal(1, model.DetachCalls);
        Assert.Equal(0, sink.SetString(1, "late"));
        Assert.Equal("Edited", adapter.Name);
        Assert.True(sink.SetString(999, "invalid") < 0);
        Assert.True(sink.SetInteger(1, 42) < 0);
        Assert.True(sink.AddString(999, "invalid") < 0);
        adapter.Dispose();
        Assert.Equal(1, model.DetachCalls);
    }

    [Fact]
    public void Writable_properties_commit_successful_non_echoing_writes_and_can_return_to_initial_value()
    {
        var generatedModel = new NonEchoingModel();
        using var generated = new SampleViewModelAdapter(generatedModel, action => action());
        var generatedChanges = new List<string?>();
        generated.PropertyChanged += (_, args) => generatedChanges.Add(args.PropertyName);

        generated.NewItem = "Error";
        generated.NewItem = "";

        Assert.Equal(["Error", ""], generatedModel.StringWrites);
        Assert.Equal("", generated.NewItem);
        Assert.Equal([nameof(generated.NewItem), nameof(generated.NewItem)], generatedChanges);

        var dynamicModel = new NonEchoingModel();
        using var dynamic = new ReflectableRustViewModelAdapter(
            dynamicModel,
            SampleViewModelMetadata.Descriptor,
            action => action());
        var dynamicChanges = new List<string?>();
        dynamic.PropertyChanged += (_, args) => dynamicChanges.Add(args.PropertyName);

        dynamic.SetMemberValue(nameof(generated.NewItem), "Error");
        dynamic.SetMemberValue(nameof(generated.NewItem), "");

        Assert.Equal(["Error", ""], dynamicModel.StringWrites);
        Assert.Equal("", dynamic.GetMemberValue(nameof(generated.NewItem)));
        Assert.Equal([nameof(generated.NewItem), nameof(generated.NewItem)], dynamicChanges);
    }

    [Fact]
    public void Compiled_two_way_text_binding_writes_when_returning_to_initial_empty_value()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new NonEchoingModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var textBox = new TextBox { DataContext = adapter };
        using var binding = textBox.Bind(
            TextBox.TextProperty,
            CompiledBinding.Create<SampleViewModelAdapter, string>(
                viewModel => viewModel.NewItem,
                mode: BindingMode.TwoWay));

        textBox.Text = "Error";
        textBox.Text = "";

        Assert.Equal(["Error", ""], model.StringWrites);
    }

    [Fact]
    public void Compiled_axaml_text_binding_writes_when_returning_to_initial_empty_value()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new NonEchoingModel();
        var window = new RustVmWindow(model);
        var textBox = window.FindControl<TextBox>("NewItemTextBox")!;

        textBox.Text = "Error";
        textBox.Text = "";

        Assert.Equal(["Error", ""], model.StringWrites);
        window.Close();
    }

    [Fact]
    public void Writable_properties_do_not_commit_failed_non_echoing_writes()
    {
        var model = new NonEchoingModel(failWrites: true);
        using var adapter = new SampleViewModelAdapter(model, action => action());

        Assert.ThrowsAny<Exception>(() => adapter.NewItem = "Rejected");

        Assert.Equal("", adapter.NewItem);
    }

    [Fact]
    public void Synchronous_same_value_normalization_is_authoritative()
    {
        var generatedModel = new NormalizingModel();
        using var generated = new SampleViewModelAdapter(generatedModel, action => action());
        var generatedChanges = 0;
        generated.PropertyChanged += (_, _) => generatedChanges++;

        generated.Name = " abc ";

        Assert.Equal("abc", generated.Name);
        Assert.Equal(0, generatedChanges);

        var dynamicModel = new NormalizingModel();
        using var dynamic = new ReflectableRustViewModelAdapter(
            dynamicModel,
            SampleViewModelMetadata.Descriptor,
            action => action());
        var dynamicChanges = 0;
        dynamic.PropertyChanged += (_, _) => dynamicChanges++;

        dynamic.SetMemberValue(nameof(generated.Name), " abc ");

        Assert.Equal("abc", dynamic.GetMemberValue(nameof(generated.Name)));
        Assert.Equal(0, dynamicChanges);
    }

    [Fact]
    public void Reflectable_non_echoing_enum_write_retains_the_concrete_enum_type()
    {
        var model = new NonEchoingModel();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            SampleViewModelMetadata.Descriptor,
            action => action());

        adapter.SetMemberValue("Priority", 2L);

        Assert.Equal([2L], model.IntegerWrites);
        Assert.Equal(Priority.High, Assert.IsType<Priority>(adapter.GetMemberValue("Priority")));
    }

    [Fact]
    public void Synchronous_nullable_normalization_to_null_is_authoritative()
    {
        var generatedModel = new NullNormalizingModel();
        using var generated = new SampleViewModelAdapter(generatedModel, action => action());

        generated.Nickname = "   ";

        Assert.Null(generated.Nickname);

        var dynamicModel = new NullNormalizingModel();
        using var dynamic = new ReflectableRustViewModelAdapter(
            dynamicModel,
            SampleViewModelMetadata.Descriptor,
            action => action());

        dynamic.SetMemberValue(nameof(generated.Nickname), "   ");

        Assert.Null(dynamic.GetMemberValue(nameof(generated.Nickname)));
    }
    [Fact]
    public void Repeated_attach_detach_balances()
    {
        for (var index = 0; index < 100; index++)
        {
            var model = new Model();
            new SampleViewModelAdapter(model, action => action()).Dispose();
            Assert.Equal(1, model.DetachCalls);
        }
    }

    [Fact]
    public async Task Adapter_dispatches_worker_thread_sink_updates_and_ignores_late_updates()
    {
        var pending = new ConcurrentQueue<Action>();
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, pending.Enqueue);
        Drain(pending);

        await Task.Run(() => model.Sink.SetString(1, "Worker"));
        Assert.Equal("Initial", adapter.Name);
        Drain(pending);
        Assert.Equal("Worker", adapter.Name);

        var sink = model.Sink;
        adapter.Dispose();
        Assert.Equal(0, sink.SetString(1, "Late"));
        Assert.Empty(pending);
        Assert.Equal("Worker", adapter.Name);
    }

    [Fact]
    public void Adapter_supports_nullable_property_publish_and_clear()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink2 = (IAvnRustVmSink2)model.Sink;

        Assert.Null(adapter.Nickname);

        Assert.Equal(0, model.Sink.SetString(5, "Nick"));
        Assert.Equal("Nick", adapter.Nickname);

        Assert.Equal(0, sink2.SetNull(5));
        Assert.Null(adapter.Nickname);

        // Invalid-contract: SetNull on a non-nullable property is rejected explicitly.
        Assert.True(sink2.SetNull(1) < 0);

        // The managed round trip (typing clears via the dedicated command,
        // not a direct null write) also resolves to null.
        adapter.Nickname = "Temp";
        Assert.Equal("Temp", model.Nickname);
        adapter.ClearNicknameCommand.Execute(null);
        Assert.Null(adapter.Nickname);
    }

    [Fact]
    public void Adapter_supports_enum_property_round_trip()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());

        Assert.Equal(Priority.Normal, adapter.Priority);

        adapter.Priority = Priority.High;
        Assert.Equal(2L, model.Priority);

        Assert.Equal(0, model.Sink.SetInteger(6, (long)Priority.Low));
        Assert.Equal(Priority.Low, adapter.Priority);

        // Invalid-contract: an out-of-range enum value is rejected explicitly
        // rather than silently truncated or defaulted.
        Assert.True(model.Sink.SetInteger(6, 99) < 0);
    }

    [Fact]
    public void Adapter_supports_nested_view_model_attach_and_dispose()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());

        Assert.Null(adapter.Address);

        adapter.ToggleAddressCommand.Execute(null);
        Assert.NotNull(adapter.Address);
        Assert.Equal("", adapter.Address!.Street);
        Assert.Equal(0, model.LastAddress!.Sink.SetString(1, "1 Rust Way"));
        Assert.Equal("1 Rust Way", adapter.Address.Street);

        // Nested properties are edited independently through the nested
        // adapter's own model, never through the parent.
        adapter.Address.City = "Rustville";
        Assert.Equal("Rustville", model.LastAddress.StringWrites[2]);

        var firstAddress = model.LastAddress;
        adapter.ToggleAddressCommand.Execute(null);
        Assert.Null(adapter.Address);
        Assert.Equal(1, firstAddress.DetachCalls);

        // Toggling back on creates a fresh nested capability; the old one
        // must not be reused (a stale capability aliasing a new one would be
        // an ownership-contract violation).
        adapter.ToggleAddressCommand.Execute(null);
        Assert.NotSame(firstAddress, model.LastAddress);
        adapter.Dispose();
        Assert.Equal(1, model.LastAddress!.DetachCalls);
    }

    [Fact]
    public void Adapter_supports_nested_collection_add_remove_move_clear()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());

        adapter.AddTaskCommand.Execute(null);
        adapter.AddTaskCommand.Execute(null);
        Assert.Equal(2, adapter.Tasks.Count);
        var taskA = model.Tasks[0];
        var taskB = model.Tasks[1];
        Assert.Equal(0, taskA.SetString(1, "First task"));
        Assert.Equal(0, taskB.SetString(1, "Second task"));
        Assert.Equal("First task", adapter.Tasks[0].Title);
        Assert.Equal("Second task", adapter.Tasks[1].Title);

        // Nested item notifications: toggling an item's own checkbox calls
        // straight into that item's own sink, not the parent's.
        adapter.Tasks[1].Done = true;
        Assert.True(taskB.BooleanWrites[2]);

        adapter.ShuffleTasksCommand.Execute(null);
        Assert.Equal("Second task", adapter.Tasks[0].Title);
        Assert.Equal("First task", adapter.Tasks[1].Title);

        adapter.RemoveFirstTaskCommand.Execute(null);
        Assert.Single(adapter.Tasks);
        Assert.Equal("First task", adapter.Tasks[0].Title);
        Assert.Equal(1, taskB.DetachCalls); // "Second task" was shuffled to the front, then removed
        Assert.Equal(0, taskA.DetachCalls);

        adapter.AddTaskCommand.Execute(null);
        Assert.Equal(2, adapter.Tasks.Count);
        var newTask = model.Tasks[1];
        var survivors = new List<TaskItemViewModelAdapter>(adapter.Tasks);
        adapter.ClearTasksCommand.Execute(null);
        Assert.Empty(adapter.Tasks);
        Assert.Equal(1, taskA.DetachCalls);
        Assert.Equal(1, newTask.DetachCalls);
        _ = survivors;
    }

    [Fact]
    public void Adapter_supports_string_collection_insert_replace_move_clear()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink2 = (IAvnRustVmSink2)model.Sink;

        Assert.Equal(0, sink2.InsertString(1, 0, "Zeroth"));
        Assert.Equal(["Zeroth", "First"], adapter.Items);

        Assert.Equal(0, sink2.ReplaceString(1, 1, "Replaced"));
        Assert.Equal(["Zeroth", "Replaced"], adapter.Items);

        Assert.Equal(0, sink2.MoveItem(1, 1, 0));
        Assert.Equal(["Replaced", "Zeroth"], adapter.Items);

        Assert.Equal(0, sink2.RemoveAt(1, 0));
        Assert.Equal(["Zeroth"], adapter.Items);

        Assert.Equal(0, sink2.ClearCollection(1));
        Assert.Empty(adapter.Items);

        // Invalid-contract: an unknown collection ID is rejected explicitly.
        Assert.True(sink2.InsertString(999, 0, "x") < 0);
        Assert.True(sink2.RemoveAt(999, 0) < 0);
        Assert.True(sink2.MoveItem(999, 0, 0) < 0);
        Assert.True(sink2.ClearCollection(999) < 0);
    }

    [Fact]
    public void Collection_operations_reject_invalid_indices_without_mutating_generated_adapter()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink = (IAvnRustVmSink2)model.Sink;

        Assert.True(sink.InsertString(1, -1, "bad") < 0);
        Assert.True(sink.InsertString(1, 2, "bad") < 0);
        Assert.True(sink.ReplaceString(1, -1, "bad") < 0);
        Assert.True(sink.ReplaceString(1, 1, "bad") < 0);
        Assert.True(sink.RemoveAt(1, -1) < 0);
        Assert.True(sink.RemoveAt(1, 1) < 0);
        Assert.True(sink.MoveItem(1, -1, 0) < 0);
        Assert.True(sink.MoveItem(1, 0, 1) < 0);
        Assert.Equal(["First"], adapter.Items);
    }

    [Fact]
    public void Collection_operations_reject_invalid_indices_without_mutating_reflectable_adapter()
    {
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model, SampleViewModelMetadata.Descriptor, action => action());
        var sink = (IAvnRustVmSink2)model.Sink;

        Assert.True(sink.InsertString(1, -1, "bad") < 0);
        Assert.True(sink.ReplaceString(1, 1, "bad") < 0);
        Assert.True(sink.RemoveAt(1, -1) < 0);
        Assert.True(sink.MoveItem(1, 0, 1) < 0);
        var items = (ObservableCollection<object?>)((IReflectableType)adapter)
            .GetTypeInfo().GetProperty("Items")!.GetValue(adapter)!;
        Assert.Equal(new object?[] { "First" }, items);
    }

    [Fact]
    public async Task Worker_thread_nested_attach_failure_rolls_back_without_publishing_generated_adapter()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var failing = new FailingNestedViewModel();

        var result = await Task.Run(() => ((IAvnRustVmSink2)model.Sink).SetModel(7, failing));

        Assert.True(result < 0);
        Assert.Null(adapter.Address);
        Assert.Equal(1, failing.DetachCalls);
    }

    [Fact]
    public async Task Worker_thread_nested_attach_failure_rolls_back_without_publishing_reflectable_adapter()
    {
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model, SampleViewModelMetadata.Descriptor, action => action());
        var failing = new FailingNestedViewModel();

        var result = await Task.Run(() => ((IAvnRustVmSink2)model.Sink).SetModel(7, failing));

        Assert.True(result < 0);
        Assert.Null(((IReflectableType)adapter).GetTypeInfo().GetProperty("Address")!.GetValue(adapter));
        Assert.Equal(1, failing.DetachCalls);
    }

    [Fact]
    public void Generated_adapter_rolls_back_nested_adapters_after_partial_root_attach_failure()
    {
        var model = new NestedPublishingModel(failAttach: true, failDetach: false);

        Assert.ThrowsAny<Exception>(() => new SampleViewModelAdapter(model, action => action()));

        Assert.Equal(1, model.DetachCalls);
        Assert.Equal(1, model.Nested.DetachCalls);
    }

    [Fact]
    public void Reflectable_adapter_rolls_back_nested_adapters_after_partial_root_attach_failure()
    {
        var model = new NestedPublishingModel(failAttach: true, failDetach: false);

        Assert.ThrowsAny<Exception>(() => new ReflectableRustViewModelAdapter(
            model, SampleViewModelMetadata.Descriptor, action => action()));

        Assert.Equal(1, model.DetachCalls);
        Assert.Equal(1, model.Nested.DetachCalls);
    }

    [Fact]
    public void Generated_adapter_cleans_nested_adapters_when_root_detach_fails()
    {
        var model = new NestedPublishingModel(failAttach: false, failDetach: true);
        var adapter = new SampleViewModelAdapter(model, action => action());

        Assert.ThrowsAny<Exception>(adapter.Dispose);

        Assert.Equal(1, model.DetachCalls);
        Assert.Equal(1, model.Nested.DetachCalls);
    }

    [Fact]
    public void Reflectable_adapter_cleans_nested_adapters_when_root_detach_fails()
    {
        var model = new NestedPublishingModel(failAttach: false, failDetach: true);
        var adapter = new ReflectableRustViewModelAdapter(
            model, SampleViewModelMetadata.Descriptor, action => action());

        Assert.ThrowsAny<Exception>(adapter.Dispose);

        Assert.Equal(1, model.DetachCalls);
        Assert.Equal(1, model.Nested.DetachCalls);
    }

    [Fact]
    public void Dispatch_exceptions_are_returned_as_hresult_failures()
    {
        var generatedModel = new Model();
        using var generated = new SampleViewModelAdapter(generatedModel, _ => throw new InvalidOperationException());
        Assert.True(generatedModel.Sink.SetString(1, "bad") < 0);

        var dynamicModel = new Model();
        using var dynamic = new ReflectableRustViewModelAdapter(
            dynamicModel, SampleViewModelMetadata.Descriptor, _ => throw new InvalidOperationException());
        Assert.True(dynamicModel.Sink.SetString(1, "bad") < 0);
    }

    [Fact]
    public void Adapter_supports_command_can_execute_state()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink2 = (IAvnRustVmSink2)model.Sink;
        var raisedCount = 0;
        adapter.SaveCommand.CanExecuteChanged += (_, _) => raisedCount++;

        Assert.True(adapter.SaveCommand.CanExecute(null));

        Assert.Equal(0, sink2.SetCommandEnabled(3, 0));
        Assert.False(adapter.SaveCommand.CanExecute(null));
        Assert.Equal(1, raisedCount);

        // Setting the same state again must not raise a redundant notification.
        Assert.Equal(0, sink2.SetCommandEnabled(3, 0));
        Assert.Equal(1, raisedCount);

        Assert.Equal(0, sink2.SetCommandEnabled(3, 1));
        Assert.True(adapter.SaveCommand.CanExecute(null));
        Assert.Equal(2, raisedCount);

        Assert.True(sink2.SetCommandEnabled(999, 1) < 0);
    }

    [Fact]
    public void Adapter_supports_validation_error_projection()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink2 = (IAvnRustVmSink2)model.Sink;
        var raised = new List<string?>();
        adapter.ErrorsChanged += (_, args) => raised.Add(args.PropertyName);

        Assert.False(adapter.HasErrors);
        Assert.Empty(adapter.GetErrors(nameof(SampleViewModelAdapter.Name)));

        Assert.Equal(0, sink2.SetPropertyError(1, "Name cannot be empty."));
        Assert.True(adapter.HasErrors);
        Assert.Equal(
            ["Name cannot be empty."],
            adapter.GetErrors(nameof(SampleViewModelAdapter.Name)).Cast<string>().ToList());
        Assert.Equal([nameof(SampleViewModelAdapter.Name)], raised);

        Assert.Equal(0, sink2.SetPropertyError(1, null));
        Assert.False(adapter.HasErrors);
        Assert.Equal([nameof(SampleViewModelAdapter.Name), nameof(SampleViewModelAdapter.Name)], raised);

        Assert.True(sink2.SetPropertyError(999, "bad") < 0);
    }

    [Fact]
    public void Disposing_adapter_disposes_nested_model_and_collection_items()
    {
        var model = new Model();
        var adapter = new SampleViewModelAdapter(model, action => action());

        adapter.ToggleAddressCommand.Execute(null);
        adapter.AddTaskCommand.Execute(null);
        adapter.AddTaskCommand.Execute(null);

        var address = model.LastAddress!;
        var tasks = new List<FakeNestedViewModel>(model.Tasks);

        adapter.Dispose();

        Assert.Equal(1, address.DetachCalls);
        Assert.All(tasks, task => Assert.Equal(1, task.DetachCalls));

        // Disposing again must stay a no-op (no double detach/double dispose).
        adapter.Dispose();
        Assert.Equal(1, address.DetachCalls);
        Assert.All(tasks, task => Assert.Equal(1, task.DetachCalls));
    }

    [Fact]
    public void Reflectable_adapter_supports_writable_properties_commands_and_collections()
    {
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            SampleViewModelMetadata.Descriptor,
            action => action());
        var typeInfo = ((IReflectableType)adapter).GetTypeInfo();
        var name = typeInfo.GetProperty("Name")!;
        var increment = typeInfo.GetProperty("IncrementCommand")!;
        var items = typeInfo.GetProperty("Items")!;
        Assert.Equal("Initial", name.GetValue(adapter));
        name.SetValue(adapter, "Edited through binding");
        Assert.Equal("Edited through binding", model.Name);
        Assert.Equal("Edited through binding", name.GetValue(adapter));

        ((ICommand)increment.GetValue(adapter)!).Execute(null);
        Assert.Equal(1, model.IncrementCalls);
        Assert.Equal(new object?[] { "First" }, (IEnumerable<object?>)items.GetValue(adapter)!);
        Assert.Null(typeInfo.GetProperty("Unknown"));
        Assert.False(typeInfo.GetProperty("Count")!.CanWrite);
    }

    [Fact]
    public async Task Reflectable_adapter_dispatches_worker_updates_and_ignores_late_updates()
    {
        var pending = new ConcurrentQueue<Action>();
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            SampleViewModelMetadata.Descriptor,
            pending.Enqueue);
        Drain(pending);
        var name = ((IReflectableType)adapter).GetTypeInfo().GetProperty("Name")!;

        await Task.Run(() => model.Sink.SetString(1, "Worker"));
        Assert.Equal("Initial", name.GetValue(adapter));
        Drain(pending);
        Assert.Equal("Worker", name.GetValue(adapter));

        var sink = model.Sink;
        adapter.Dispose();
        Assert.Equal(0, sink.SetString(1, "Late"));
        Assert.Empty(pending);
        Assert.Equal("Worker", name.GetValue(adapter));
    }

    [Fact]
    public void Reflectable_adapter_supports_nullable_enum_and_nested_view_model()
    {
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            SampleViewModelMetadata.Descriptor,
            action => action());
        var typeInfo = ((IReflectableType)adapter).GetTypeInfo();
        var nickname = typeInfo.GetProperty("Nickname")!;
        var priority = typeInfo.GetProperty("Priority")!;
        var address = typeInfo.GetProperty("Address")!;
        var sink2 = (IAvnRustVmSink2)model.Sink;

        Assert.Null(nickname.GetValue(adapter));
        Assert.Equal(0, model.Sink.SetString(5, "Nick"));
        Assert.Equal("Nick", nickname.GetValue(adapter));
        Assert.Equal(0, sink2.SetNull(5));
        Assert.Null(nickname.GetValue(adapter));

        Assert.IsType<Priority>(priority.GetValue(adapter));
        Assert.Equal(Priority.Normal, priority.GetValue(adapter));
        Assert.Equal(0, model.Sink.SetInteger(6, (long)Priority.High));
        Assert.Equal(Priority.High, priority.GetValue(adapter));

        Assert.Null(address.GetValue(adapter));
        var nested = new FakeNestedViewModel();
        Assert.Equal(0, sink2.SetModel(7, nested));
        Assert.IsType<ReflectableRustViewModelAdapter>(address.GetValue(adapter));
        Assert.Equal(0, sink2.SetModel(7, null));
        Assert.Null(address.GetValue(adapter));
        Assert.Equal(1, nested.DetachCalls);
    }

    [Fact]
    public void Reflectable_adapter_supports_nested_collection_operations()
    {
        var model = new Model();
        using var adapter = new ReflectableRustViewModelAdapter(
            model,
            SampleViewModelMetadata.Descriptor,
            action => action());
        var typeInfo = ((IReflectableType)adapter).GetTypeInfo();
        var tasks = (System.Collections.ObjectModel.ObservableCollection<object?>)
            typeInfo.GetProperty("Tasks")!.GetValue(adapter)!;
        var sink2 = (IAvnRustVmSink2)model.Sink;

        var first = new FakeNestedViewModel();
        var second = new FakeNestedViewModel();
        Assert.Equal(0, sink2.AddModel(2, first));
        Assert.Equal(0, sink2.AddModel(2, second));
        Assert.Equal(2, tasks.Count);

        Assert.Equal(0, sink2.MoveItem(2, 0, 1));

        Assert.Equal(0, sink2.RemoveAt(2, 0));
        Assert.Single(tasks);

        Assert.Equal(0, sink2.ClearCollection(2));
        Assert.Empty(tasks);
        Assert.Equal(1, first.DetachCalls);
        Assert.Equal(1, second.DetachCalls);
    }

    [Fact]
    public void Invalid_property_and_command_ids_return_explicit_errors_not_partial_state()
    {
        var model = new Model();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var sink2 = (IAvnRustVmSink2)model.Sink;

        Assert.True(model.Sink.SetString(999, "x") < 0);
        Assert.True(model.Sink.SetInteger(999, 0) < 0);
        Assert.True(model.Sink.SetBoolean(999, 0) < 0);
        Assert.True(model.Sink.SetDouble(999, 0) < 0);
        Assert.True(model.Sink.AddString(999, "x") < 0);
        Assert.True(sink2.SetNull(999) < 0);
        Assert.True(sink2.SetModel(999, null) < 0);
        Assert.True(sink2.AddModel(999, null) < 0);
        Assert.True(sink2.InsertModel(999, 0, null) < 0);
        Assert.True(sink2.ReplaceModel(999, 0, null) < 0);
        Assert.True(sink2.SetCommandEnabled(999, 1) < 0);
        Assert.True(sink2.SetPropertyError(999, "x") < 0);

        // None of the invalid calls perturbed valid state.
        Assert.Equal("Initial", adapter.Name);
        Assert.Equal(["First"], adapter.Items);
    }

    private static void Drain(ConcurrentQueue<Action> pending)
    {
        while (pending.TryDequeue(out var action))
            action();
    }

    private sealed class FakeNestedViewModel : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;

        public IAvnRustVmSink Sink => _sink!;
        public int DetachCalls { get; private set; }
        public Dictionary<int, string> StringWrites { get; } = [];
        public Dictionary<int, bool> BooleanWrites { get; } = [];

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = sink;
            return 0;
        }

        public int Detach()
        {
            DetachCalls++;
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value)
        {
            StringWrites[propertyId] = value ?? "";
            _sink!.SetString(propertyId, value);
            return 0;
        }

        public int SetInteger(int propertyId, long value) => unchecked((int)0x80070057);

        public int SetBoolean(int propertyId, int value)
        {
            BooleanWrites[propertyId] = value != 0;
            _sink!.SetBoolean(propertyId, value);
            return 0;
        }

        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);
        public int Execute(int commandId, string? parameter) => unchecked((int)0x80070057);
        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class NonEchoingModel(bool failWrites = false) : IAvnRustViewModel
    {
        public List<string> StringWrites { get; } = [];
        public List<long> IntegerWrites { get; } = [];

        public int Attach(IAvnRustVmSink? sink) => 0;
        public int Detach() => 0;

        public int SetString(int propertyId, string? value)
        {
            if (propertyId != 3)
                return unchecked((int)0x80070057);
            if (failWrites)
                return unchecked((int)0x80004005);
            StringWrites.Add(value ?? "");
            return 0;
        }

        public int SetInteger(int propertyId, long value)
        {
            if (propertyId != 6)
                return unchecked((int)0x80070057);
            IntegerWrites.Add(value);
            return 0;
        }
        public int SetBoolean(int propertyId, int value) => unchecked((int)0x80070057);
        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);
        public int Execute(int commandId, string? parameter) => unchecked((int)0x80070057);
        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class NormalizingModel : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = sink;
            return sink!.SetString(1, "abc");
        }

        public int Detach()
        {
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value) =>
            propertyId == 1
                ? _sink!.SetString(1, value?.Trim())
                : unchecked((int)0x80070057);

        public int SetInteger(int propertyId, long value) => unchecked((int)0x80070057);
        public int SetBoolean(int propertyId, int value) => unchecked((int)0x80070057);
        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);
        public int Execute(int commandId, string? parameter) => unchecked((int)0x80070057);
        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class NullNormalizingModel : IAvnRustViewModel
    {
        private IAvnRustVmSink2? _sink;

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = (IAvnRustVmSink2)sink!;
            return 0;
        }

        public int Detach()
        {
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value) =>
            propertyId == 5
                ? _sink!.SetNull(5)
                : unchecked((int)0x80070057);

        public int SetInteger(int propertyId, long value) => unchecked((int)0x80070057);
        public int SetBoolean(int propertyId, int value) => unchecked((int)0x80070057);
        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);
        public int Execute(int commandId, string? parameter) => unchecked((int)0x80070057);
        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class FailingNestedViewModel : IAvnRustViewModel
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

    private sealed class NestedPublishingModel(bool failAttach, bool failDetach) : IAvnRustViewModel
    {
        public FakeNestedViewModel Nested { get; } = new();
        public int DetachCalls { get; private set; }

        public int Attach(IAvnRustVmSink? sink)
        {
            var result = ((IAvnRustVmSink2)sink!).AddModel(2, Nested);
            return result < 0 || failAttach ? unchecked((int)0x80004005) : 0;
        }

        public int Detach()
        {
            DetachCalls++;
            return failDetach ? unchecked((int)0x80004005) : 0;
        }

        public int SetString(int propertyId, string? value) => unchecked((int)0x80070057);
        public int SetInteger(int propertyId, long value) => unchecked((int)0x80070057);
        public int SetBoolean(int propertyId, int value) => unchecked((int)0x80070057);
        public int SetDouble(int propertyId, double value) => unchecked((int)0x80070057);
        public int Execute(int commandId, string? parameter) => unchecked((int)0x80070057);
        public int BeginAsync(int commandId, string? parameter) => unchecked((int)0x80070057);
    }

    private sealed class Model : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;
        private bool _addressSet;
        private int _taskCount;

        public string Name { get; private set; } = "";

        public int IncrementCalls { get; private set; }

        public List<string> AddedItems { get; } = [];

        public int SaveCalls { get; private set; }

        public int DetachCalls { get; private set; }

        public string? Nickname { get; private set; }

        public long Priority { get; private set; } = 1;

        public FakeNestedViewModel? LastAddress { get; private set; }

        public List<FakeNestedViewModel> Tasks { get; } = [];

        public IAvnRustVmSink Sink => _sink!;

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
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value)
        {
            switch (propertyId)
            {
                case 1:
                    Name = value ?? "";
                    _sink!.SetString(1, Name);
                    return 0;
                case 3:
                    _sink!.SetString(3, value);
                    return 0;
                case 5:
                    Nickname = value;
                    _sink!.SetString(5, value);
                    return 0;
                case 8:
                    _sink!.SetString(8, value);
                    return 0;
                default:
                    return unchecked((int)0x80070057);
            }
        }

        public int SetInteger(int propertyId, long value)
        {
            if (propertyId != 6)
                return unchecked((int)0x80070057);
            Priority = value;
            _sink!.SetInteger(6, value);
            return 0;
        }

        public int SetBoolean(int propertyId, int value) =>
            unchecked((int)0x80070057);

        public int SetDouble(int propertyId, double value) =>
            unchecked((int)0x80070057);

        public int Execute(int commandId, string? parameter)
        {
            var sink2 = (IAvnRustVmSink2)_sink!;
            switch (commandId)
            {
                case 1:
                    IncrementCalls++;
                    return 0;
                case 2:
                    AddedItems.Add(parameter ?? "");
                    _sink!.AddString(1, parameter);
                    _sink.SetString(3, "");
                    return 0;
                case 4:
                    Nickname = null;
                    sink2.SetNull(5);
                    return 0;
                case 5:
                    _addressSet = !_addressSet;
                    if (_addressSet)
                    {
                        var address = new FakeNestedViewModel();
                        LastAddress = address;
                        sink2.SetModel(7, address);
                    }
                    else
                    {
                        sink2.SetModel(7, null);
                    }
                    return 0;
                case 6:
                    _taskCount++;
                    var task = new FakeNestedViewModel();
                    Tasks.Add(task);
                    sink2.AddModel(2, task);
                    return 0;
                case 7:
                    if (_taskCount == 0)
                        return 0;
                    _taskCount--;
                    Tasks.RemoveAt(0);
                    sink2.RemoveAt(2, 0);
                    return 0;
                case 8:
                    if (_taskCount < 2)
                        return 0;
                    sink2.MoveItem(2, _taskCount - 1, 0);
                    var moved = Tasks[^1];
                    Tasks.RemoveAt(Tasks.Count - 1);
                    Tasks.Insert(0, moved);
                    return 0;
                case 9:
                    _taskCount = 0;
                    Tasks.Clear();
                    sink2.ClearCollection(2);
                    return 0;
                default:
                    return unchecked((int)0x80070057);
            }
        }

        public int BeginAsync(int commandId, string? parameter)
        {
            Assert.Equal(3, commandId);
            SaveCalls++;
            return 0;
        }
    }
}
