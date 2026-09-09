using System.Collections.Specialized;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Rust;
using Avalonia.Rust.Sample.Generated;
using Xunit;

namespace Avalonia.Host.Tests;

public class RustVmTableTests
{
    [Fact]
    public void Generated_table_metadata_constructs_resizable_tableview_columns()
    {
        var columns = SampleViewModelMetadata.CreateTraceRowsTableColumns();

        Assert.Equal(4, columns.Count);
        Assert.All(columns, column => Assert.True(column.CanUserResize));
        Assert.Equal("Timestamp", columns[0].Header);
        Assert.Equal("Message", columns[3].Header);
        Assert.True(columns[3].Width.IsStar);
        Assert.Equal("Event.Source", SampleViewModelMetadata.TraceRowsTable.Columns[2].Path);
    }

    [Fact]
    public void Large_atomic_snapshot_raises_exactly_one_reset_without_creating_controls()
    {
        var rows = new BatchObservableCollection<object?>();
        var resetCount = 0;
        rows.CollectionChanged += (_, e) =>
        {
            if (e.Action == NotifyCollectionChangedAction.Reset)
                resetCount++;
        };

        rows.ReplaceSnapshot(Enumerable.Range(0, 100_000).Select(index => (object?)index).ToArray());

        Assert.Equal(100_000, rows.Count);
        Assert.Equal(1, resetCount);
        Assert.Empty(new TableView().GetRealizedContainers());
    }
}
