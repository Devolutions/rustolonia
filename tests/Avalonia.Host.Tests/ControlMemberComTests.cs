using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Host.Com;
using Avalonia.Layout;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers the completeness members published on the widened <c>IAvnContentControl</c>,
/// <c>IAvnButton</c>, <c>IAvnToggleButton</c>, <c>IAvnListBox</c> and <c>IAvnComboBox</c>
/// vtables. Every assertion goes through a real CCW/RCW round trip and then reads the
/// Avalonia object, so a member that only updated wrapper state would fail.
/// </summary>
public unsafe class ControlMemberComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Content_control_alignment_round_trips_and_reaches_the_avalonia_object()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateContentControl(out var projected));
        Assert.NotNull(projected);

        Through<IAvnContentControl>(projected, contentControl =>
        {
            Assert.Equal(0, contentControl.GetHorizontalContentAlignment(out var initial));
            Assert.Equal((int)HorizontalAlignment.Stretch, initial);

            Assert.Equal(
                0,
                contentControl.SetHorizontalContentAlignment((int)HorizontalAlignment.Center));
            Assert.Equal(0, contentControl.GetHorizontalContentAlignment(out var horizontal));
            Assert.Equal((int)HorizontalAlignment.Center, horizontal);

            Assert.Equal(
                0,
                contentControl.SetVerticalContentAlignment((int)VerticalAlignment.Bottom));
            Assert.Equal(0, contentControl.GetVerticalContentAlignment(out var vertical));
            Assert.Equal((int)VerticalAlignment.Bottom, vertical);
        });

        var value = Target<ContentControl>(projected);
        Assert.Equal(HorizontalAlignment.Center, value.HorizontalContentAlignment);
        Assert.Equal(VerticalAlignment.Bottom, value.VerticalContentAlignment);
    }

    [Fact]
    public void Button_members_round_trip_and_is_pressed_is_read_only()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnButton>(projected, button =>
        {
            Assert.Equal(0, button.GetClickMode(out var initialClickMode));
            Assert.Equal((int)ClickMode.Release, initialClickMode);

            Assert.Equal(0, button.SetClickMode((int)ClickMode.Press));
            Assert.Equal(0, button.GetClickMode(out var clickMode));
            Assert.Equal((int)ClickMode.Press, clickMode);

            Assert.Equal(0, button.SetIsDefault(1));
            Assert.Equal(0, button.GetIsDefault(out var isDefault));
            Assert.Equal(1, isDefault);

            Assert.Equal(0, button.SetIsCancel(1));
            Assert.Equal(0, button.GetIsCancel(out var isCancel));
            Assert.Equal(1, isCancel);

            // IsPressed is raised by Avalonia's own input handling, so the ABI publishes a
            // getter and no setter.
            Assert.Equal(0, button.GetIsPressed(out var isPressed));
            Assert.Equal(0, isPressed);
        });

        Assert.DoesNotContain(
            typeof(IAvnButton).GetMethods(),
            method => method.Name == "SetIsPressed");

        var value = Target<Button>(projected);
        Assert.Equal(ClickMode.Press, value.ClickMode);
        Assert.True(value.IsDefault);
        Assert.True(value.IsCancel);
    }

    [Fact]
    public void Toggle_button_is_three_state_round_trips_alongside_is_checked()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateToggleButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnToggleButton>(projected, toggle =>
        {
            Assert.Equal(0, toggle.GetIsThreeState(out var initial));
            Assert.Equal(0, initial);

            Assert.Equal(0, toggle.SetIsThreeState(1));
            Assert.Equal(0, toggle.GetIsThreeState(out var isThreeState));
            Assert.Equal(1, isThreeState);
        });

        Assert.True(Target<ToggleButton>(projected).IsThreeState);
    }

    [Fact]
    public void List_box_selection_mode_and_selection_commands_reach_the_avalonia_object()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateListBox(out var projected));
        Assert.NotNull(projected);
        var listBox = Target<ListBox>(projected);
        listBox.ItemsSource = new[] { "first", "second", "third" };

        Through<IAvnListBox>(projected, box =>
        {
            Assert.Equal(0, box.GetSelectionMode(out var initial));
            Assert.Equal((int)SelectionMode.Single, initial);

            Assert.Equal(0, box.SetSelectionMode((int)SelectionMode.Multiple));
            Assert.Equal(0, box.GetSelectionMode(out var selectionMode));
            Assert.Equal((int)SelectionMode.Multiple, selectionMode);

            Assert.Equal(0, box.SelectAll());
            Assert.Equal(3, listBox.Selection.Count);

            Assert.Equal(0, box.UnselectAll());
            Assert.Equal(0, listBox.Selection.Count);
        });

        Assert.Equal(SelectionMode.Multiple, listBox.SelectionMode);
    }

    [Fact]
    public void Combo_box_drop_down_members_round_trip()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateComboBox(out var projected));
        Assert.NotNull(projected);

        Through<IAvnComboBox>(projected, comboBox =>
        {
            Assert.Equal(0, comboBox.GetMaxDropDownHeight(out var initialHeight));
            Assert.Equal(200d, initialHeight);

            Assert.Equal(0, comboBox.SetIsDropDownOpen(1));
            Assert.Equal(0, comboBox.GetIsDropDownOpen(out var isDropDownOpen));
            Assert.Equal(1, isDropDownOpen);

            Assert.Equal(0, comboBox.SetIsEditable(1));
            Assert.Equal(0, comboBox.GetIsEditable(out var isEditable));
            Assert.Equal(1, isEditable);

            Assert.Equal(0, comboBox.SetMaxDropDownHeight(320));
            Assert.Equal(0, comboBox.GetMaxDropDownHeight(out var maxDropDownHeight));
            Assert.Equal(320d, maxDropDownHeight);
        });

        var value = Target<ComboBox>(projected);
        Assert.True(value.IsDropDownOpen);
        Assert.True(value.IsEditable);
        Assert.Equal(320d, value.MaxDropDownHeight);
    }

    private static void Through<T>(object wrapper, Action<T> body) where T : class
    {
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(wrapper, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);
        try
        {
            body((T)s_wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None));
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
