//! Tests for hierarchy upcasting (`Upcast`, `Deref`), ergonomic value conversions
//! (`From`/`Into`, `IntoValue`, `IntoProp`), fine-grained reactivity (`Signal`,
//! `Memo`, `Effect`, `NodeRef`, `TwoWay`), `El<T>`, `rsx!`, and `reactive_view_model!`.

use rustolonia::{
    prop, reactive_view_model, rsx, AsControl, AvaloniaObject, Border, Brush, Button, CheckBox,
    Color, ContentControl, Control, CornerRadius, Dock, Effect, El, IntoProp, IntoValue, ListBox,
    ListBoxItem, Memo, NodeRef, Orientation, Paint, PixelPoint, PlacementMode, Point, PropBinding,
    Rect, Result, Signal, Size, Slider, StackPanel, StyledElement, TemplatedControl, TextBlock,
    TextBox, Thickness, ToggleSwitch, Upcast, Vector, Window,
};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[test]
fn geometry_and_color_ergonomic_conversions_match_constructors() {
    assert_eq!(Thickness::from(8.0), Thickness::uniform(8.0));
    assert_eq!(
        Thickness::from((12.0, 4.0)),
        Thickness::symmetric(12.0, 4.0)
    );
    assert_eq!(
        Thickness::from((1.0, 2.0, 3.0, 4.0)),
        Thickness::new(1.0, 2.0, 3.0, 4.0)
    );
    assert_eq!(
        Thickness::from([1.0, 2.0, 3.0, 4.0]),
        Thickness::new(1.0, 2.0, 3.0, 4.0)
    );

    assert_eq!(CornerRadius::from(6.0), CornerRadius::uniform(6.0));
    assert_eq!(
        CornerRadius::from((1.0, 2.0, 3.0, 4.0)),
        CornerRadius::new(1.0, 2.0, 3.0, 4.0)
    );
    assert_eq!(
        CornerRadius::from([1.0, 2.0, 3.0, 4.0]),
        CornerRadius::new(1.0, 2.0, 3.0, 4.0)
    );

    assert_eq!(Size::from((320.0, 240.0)), Size::new(320.0, 240.0));
    assert_eq!(Size::from([320.0, 240.0]), Size::new(320.0, 240.0));
    assert_eq!(Point::from((10.0, 20.0)), Point::new(10.0, 20.0));
    assert_eq!(Vector::from((-5.0, 15.0)), Vector::new(-5.0, 15.0));
    assert_eq!(
        Rect::from((1.0, 2.0, 100.0, 50.0)),
        Rect::new(1.0, 2.0, 100.0, 50.0)
    );
    assert_eq!(
        Rect::from((Point::new(1.0, 2.0), Size::new(100.0, 50.0))),
        Rect::new(1.0, 2.0, 100.0, 50.0)
    );
    assert_eq!(PixelPoint::from((100, 200)), PixelPoint::new(100, 200));
    assert_eq!(PixelPoint::from([100, 200]), PixelPoint::new(100, 200));

    assert_eq!(
        Color::from((0x12, 0x34, 0x56)),
        Color::rgb(0x12, 0x34, 0x56)
    );
    assert_eq!(
        Color::from((0x80, 0x12, 0x34, 0x56)),
        Color::new(0x80, 0x12, 0x34, 0x56)
    );
    assert_eq!(
        Color::from(0x8012_3456_u32),
        Color::new(0x80, 0x12, 0x34, 0x56)
    );

    assert_eq!(Color::parse_hex("#09F"), Some(Color::rgb(0x00, 0x99, 0xFF)));
    assert_eq!(
        Color::parse_hex("#809F"),
        Some(Color::new(0x88, 0x00, 0x99, 0xFF))
    );
    assert_eq!(
        Color::parse_hex("#007ACC"),
        Some(Color::rgb(0x00, 0x7A, 0xCC))
    );
    assert_eq!(
        Color::parse_hex("80007ACC"),
        Some(Color::new(0x80, 0x00, 0x7A, 0xCC))
    );
    assert_eq!(Color::parse_hex("#GGGGGG"), None);
    assert_eq!(Color::parse_hex("#12"), None);

    assert_eq!(
        Brush::from((0x00, 0x7A, 0xCC)),
        Brush::solid(Color::rgb(0x00, 0x7A, 0xCC))
    );
    assert_eq!(
        Paint::from((0x00, 0x7A, 0xCC)),
        Paint::Solid(Brush::solid(Color::rgb(0x00, 0x7A, 0xCC)))
    );

    let opt_brush: Option<Brush> = (0x10_u8, 0x20_u8, 0x30_u8).into_value();
    assert_eq!(opt_brush, Some(Brush::solid(Color::rgb(0x10, 0x20, 0x30))));
}

#[test]
fn hierarchy_traits_upcast_and_deref_are_implemented_across_lineage() {
    fn assert_upcast<Sub, Ancestor>()
    where
        Sub: Upcast<Ancestor> + AsRef<Ancestor>,
    {
    }

    fn assert_deref<Sub, Parent>()
    where
        Sub: Deref<Target = Parent>,
    {
    }

    fn assert_as_control<T: AsControl>() {}

    assert_upcast::<Button, ContentControl>();
    assert_upcast::<Button, TemplatedControl>();
    assert_upcast::<Button, Control>();
    assert_upcast::<Button, StyledElement>();
    assert_upcast::<Button, AvaloniaObject>();

    assert_deref::<Button, ContentControl>();
    assert_deref::<ContentControl, TemplatedControl>();
    assert_deref::<TemplatedControl, Control>();
    assert_deref::<Control, StyledElement>();
    assert_deref::<StyledElement, AvaloniaObject>();

    assert_as_control::<Button>();
    assert_as_control::<&Button>();
}

#[test]
fn signals_memos_and_effects_track_dependencies_and_deduplicate_updates() {
    let count = Signal::new(0_i32);
    let show_doubled = Signal::new(true);
    let doubled = Memo::new(move || count.get() * 2);

    let observed = Rc::new(RefCell::new(Vec::new()));
    let observed_for_effect = observed.clone();
    let effect = Effect::new(move || {
        let value = if show_doubled.get() {
            doubled.get()
        } else {
            -1
        };
        observed_for_effect.borrow_mut().push(value);
        Ok(())
    })
    .expect("initial effect run");

    assert_eq!(*observed.borrow(), vec![0]);

    count.set(3);
    assert_eq!(doubled.get_untracked(), 6);
    assert_eq!(*observed.borrow(), vec![0, 6]);

    // Setting the same value with `set_if_changed` does not re-trigger subscribers.
    count.set_if_changed(3);
    assert_eq!(*observed.borrow(), vec![0, 6]);

    // Switching `show_doubled` to false unsubscribes the effect from `doubled`.
    show_doubled.set(false);
    assert_eq!(*observed.borrow(), vec![0, 6, -1]);

    count.set(10);
    assert_eq!(*observed.borrow(), vec![0, 6, -1]);

    // Disposing stops future runs even when `show_doubled` changes.
    effect.dispose();
    show_doubled.set(true);
    assert_eq!(*observed.borrow(), vec![0, 6, -1]);
}

#[test]
fn into_prop_accepts_values_signals_memos_closures_and_two_way_bindings() {
    let static_prop: PropBinding<Thickness> = 12.0.into_prop();
    assert!(matches!(static_prop, PropBinding::Static(t) if t == Thickness::uniform(12.0)));

    let count = Signal::new(5_i32);
    let signal_prop: PropBinding<i32> = count.into_prop();
    match signal_prop {
        PropBinding::Reactive(mut f) => assert_eq!(f(), 5),
        _ => panic!("expected Reactive binding"),
    }

    let memo = Memo::new(move || format!("Count={}", count.get()));
    let memo_prop: PropBinding<String> = memo.into_prop();
    match memo_prop {
        PropBinding::Reactive(mut f) => assert_eq!(f(), "Count=5"),
        _ => panic!("expected Reactive binding"),
    }

    let closure_prop: PropBinding<String> =
        (move || format!("Next={}", count.get() + 1)).into_prop();
    match closure_prop {
        PropBinding::Reactive(mut f) => assert_eq!(f(), "Next=6"),
        _ => panic!("expected Reactive binding"),
    }

    let flag = Signal::new(true);
    let two_way_opt: PropBinding<Option<bool>> = flag.two_way().into_prop();
    assert!(matches!(two_way_opt, PropBinding::TwoWay(_)));
}

#[test]
fn rsx_macro_builds_deferred_element_trees_with_attached_props_and_control_flow() {
    let name = Signal::new(String::from("Alice"));
    let checked = Signal::new(true);
    let slider_val = Signal::new(42.0);
    let show_extra = true;
    let items = ["One", "Two", "Three"];
    let input_ref = NodeRef::<TextBox>::new();

    let _window: El<Window> = rsx! {
        Window {
            title: "Declarative RSX Window",
            width: 640.0,
            height: 480.0,
            Border {
                padding: (16.0, 12.0),
                corner_radius: 8.0,
                background: (0x1E, 0x1E, 0x2E),
                StackPanel {
                    orientation: Orientation::Vertical,
                    spacing: 10.0,
                    margin: 24.0,
                    TextBlock {
                        class: "heading primary",
                        font_size: 22.0,
                        text: move || format!("Hello, {}!", name.get()),
                    }
                    TextBox {
                        ref: input_ref,
                        Grid.row: 0,
                        Grid.Column: 1,
                        Grid.ColumnSpan: 2,
                        DockPanel.dock: Dock::Top,
                        ToolTip.tip: "Enter your full name",
                        ToolTip.placement: PlacementMode::Bottom,
                        placeholder_text: "Your name",
                        text: name.two_way(),
                    }
                    CheckBox {
                        checked: checked.two_way(),
                        "Enable notifications"
                    }
                    ToggleSwitch {
                        checked: checked.two_way(),
                        "Dark mode"
                    }
                    Slider {
                        minimum: 0.0,
                        maximum: 100.0,
                        value: slider_val.two_way(),
                    }
                    if show_extra {
                        TextBlock { "Extra section is visible" }
                    } else {
                        TextBlock { "Hidden" }
                    }
                    ListBox {
                        for label in items {
                            ListBoxItem { { label } }
                        }
                    }
                    Button {
                        on_click: move |_| name.set(String::new()),
                        "Clear"
                    }
                }
            }
        }
    };
    assert!(input_ref.get().is_none());
}

#[derive(Clone, Default)]
struct MockSink {
    events: Rc<RefCell<Vec<String>>>,
}

impl MockSink {
    fn set_name(&self, value: &str) -> Result<()> {
        self.events.borrow_mut().push(format!("name={value}"));
        Ok(())
    }

    fn set_count(&self, value: i64) -> Result<()> {
        self.events.borrow_mut().push(format!("count={value}"));
        Ok(())
    }

    fn set_nickname(&self, value: Option<&str>) -> Result<()> {
        self.events.borrow_mut().push(format!("nickname={value:?}"));
        Ok(())
    }
}

reactive_view_model! {
    pub struct DemoViewModel (sink: MockSink) {
        pub name: String = String::from("Initial") => set_name(str_ref),
        pub count: i64 = 1 => set_count(by_val),
        pub nickname: Option<String> = None => set_nickname(opt_str),
    }
}

#[test]
fn reactive_view_model_and_prop_macros_forward_and_deduplicate_sink_updates() {
    let mut vm = DemoViewModel::new();
    let sink = MockSink::default();

    vm.attach_sink(sink.clone()).expect("attach");
    assert_eq!(
        *sink.events.borrow(),
        vec!["name=Initial", "count=1", "nickname=None"]
    );

    // Setting the same value is deduplicated via PartialEq and does not call the sink.
    vm.set_count(1).expect("no-op set_count");
    assert_eq!(sink.events.borrow().len(), 3);

    vm.set_count(2).expect("updated set_count");
    vm.set_name("Updated").expect("updated set_name");
    vm.set_nickname(Some(String::from("Ace")))
        .expect("updated set_nickname");
    assert_eq!(
        *sink.events.borrow(),
        vec![
            "name=Initial",
            "count=1",
            "nickname=None",
            "count=2",
            "name=Updated",
            "nickname=Some(\"Ace\")",
        ]
    );

    let count_prop = prop!(mut DemoViewModel => count, set_count);
    assert_eq!(count_prop.name, "count");
    assert_eq!(count_prop.read(&vm), 2);
    count_prop
        .write(&mut vm, 10)
        .expect("write via PropertyPath");
    assert_eq!(*vm.count(), 10);
    assert_eq!(sink.events.borrow().last().unwrap(), "count=10");

    vm.detach_sink().expect("detach");
    assert!(vm.sink().is_none());
    vm.set_count(99).expect("update while detached");
    assert_eq!(sink.events.borrow().len(), 7);
}
