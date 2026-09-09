using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class WaveU19DataTemplateComTests
{
    [Fact]
    public void Item_template_round_trips_through_the_com_bridge()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);

        var wrapper = Assert.IsType<AvnListBox>(projected);

        Assert.Equal(0, wrapper.GetItemTemplate(out var none));
        Assert.Null(none);

        var template = AvnDataTemplate.FromTemplate(new RecyclingTemplate());
        Assert.Equal(0, wrapper.SetItemTemplate(template));
        Assert.Equal(0, wrapper.GetItemTemplate(out var read));
        Assert.NotNull(read);

        // The managed side sees the adapter wrapping the CCW round trip.
        var value = Assert.IsType<ListBox>(
            typeof(AvnListBox)
                .GetProperty("_value", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(wrapper));
        Assert.NotNull(value.ItemTemplate);
    }

    [Fact]
    public void Match_and_build_cross_through_the_foreign_template()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var template = new ForeignTemplate();

        // A foreign IAvnDataTemplate (what Rust's CCW looks like on the wire)
        // converts back to a managed IDataTemplate whose Match/Build call
        // through the adapter.
        var restored = AvnDataTemplate.ToTemplate(template)!;
        Assert.True(restored.Match("anything"));
        restored.Build(null);
        Assert.True(template.BuildCalled);
    }

    private sealed class RecyclingTemplate : Avalonia.Controls.Templates.IDataTemplate
    {
        public bool BuildCalled { get; private set; }

        public bool Match(object? data) => data is not null;

        public Control? Build() => Build(null);

        public Control? Build(object? param)
        {
            BuildCalled = true;
            return new Border();
        }
    }

    private sealed class ForeignTemplate : IAvnDataTemplate
    {
        public bool BuildCalled { get; private set; }

        public int Match(AvnVariant data, out int value)
        {
            value = data.Tag == AvnVariant.TagUtf16 ? 1 : 0;
            return 0;
        }

        public int Build(out IAvnControl? value)
        {
            BuildCalled = true;
            value = null; // The adapter tolerates a null build in the unit-test host.
            return 0;
        }
    }
}
