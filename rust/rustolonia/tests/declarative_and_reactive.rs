//! Tests for hierarchy upcasting (`Upcast`, `Deref`), ergonomic value conversions
//! (`From`/`Into`, `IntoValue`, `IntoProp`), fine-grained reactivity (`Signal`,
//! `Memo`, `Effect`, `NodeRef`, `TwoWay`), `El<T>`, `rsx!`, and `reactive_view_model!`.

use rustolonia::{
    color, component, fmt, lens, prop, provide_context, reactive_view_model, rsx, theme_brush,
    tracks, use_context, when_any, with_ui_error_handler, AnyEl, AsControl, AsyncPopulator,
    AutoCompleteBox, AvaloniaObject, Border, BoxShadow, BoxShadows, Brush, Button, CheckBox,
    CivilDate, CivilTime, Color, Command, ContentControl, Control, ControlKeyDownEventArgs,
    ControlSlot, CornerRadius, Decimal, Dock, Downcast, Effect, El, Error, FontWeight, For,
    GradientStop, Grid, GridTrack, GridTracks, IntoProp, IntoValue, ItemFilter, ItemSelector,
    ItemsView, Key, KeyModifiers, KeyedListReconciler, Lens, LinearGradientBrush, ListBox,
    ListBoxItem, Memo, NodeRef, NonOwningRef, Notification, NotificationType, Orientation, Paint,
    PhysicalKey, PixelPoint, PlacementMode, Point, Popup, PopupAnchor, PopupGravity,
    PopupPlacementCallback, PopupPlacementInput, PopupPlacementOutput,
    PopupPositionerConstraintAdjustment, PropBinding, PropertyPath, RadialGradientBrush,
    ReactiveScope, Rect, RelativePoint, RelativeScalar, RelativeUnit, Resource, ResourceState,
    Result, SelectingItemsControlSelectionChangedArgs, SelectionMode, Shortcut, Show, Signal, Size,
    Slider, SpreadMethod, StackPanel, Style, StyleSheet, StyledElement, TemplatedControl,
    TextBlock, TextBox, TextFilter, TextSelector, ThemeVariant, Thickness, ToggleSwitch, Upcast,
    Variant, Vector, ViewRegistry, Window,
};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
        Color::rgba(0x12, 0x34, 0x56, 0x80),
        Color::new(0x80, 0x12, 0x34, 0x56)
    );
    assert_eq!(
        Color::argb(0x80, 0x12, 0x34, 0x56),
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

    fn assert_downcast<Ancestor, Sub>()
    where
        Ancestor: Downcast<Sub>,
    {
    }

    assert_downcast::<AvaloniaObject, Button>();
    assert_downcast::<StyledElement, Button>();
    assert_downcast::<Control, Button>();
    assert_downcast::<TemplatedControl, Button>();
    assert_downcast::<ContentControl, Button>();
    assert_downcast::<Button, Button>();

    let ref_a = NonOwningRef::from_object_id(101, "control-a");
    let ref_a_dup = NonOwningRef::from_object_id(101, "control-a-alias");
    let ref_b = NonOwningRef::from_object_id(202, "control-b");
    assert_eq!(ref_a, ref_a_dup);
    assert_ne!(ref_a, ref_b);
    assert_eq!(ref_a.object_id(), 101);
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
fn nested_property_paths_follow_replacements_and_write_to_the_current_model() {
    #[derive(Clone)]
    struct Person {
        name: Signal<String>,
    }

    let first = Signal::new("First".to_string());
    let second = Signal::new("Second".to_string());
    let selected = Signal::new(Some(Person { name: first }));
    let path = PropertyPath::writable(
        move || {
            let person = selected
                .get()
                .ok_or_else(|| rustolonia::Error::Binding("no selected person".into()))?;
            Ok(person.name.get())
        },
        move |value| {
            let person = selected
                .get_untracked()
                .ok_or_else(|| rustolonia::Error::Binding("no selected person".into()))?;
            person.name.set_if_changed(value);
            Ok(())
        },
    );
    let observed = Rc::new(RefCell::new(Vec::new()));
    let lifetime = ReactiveScope::new();
    lifetime.run(|| {
        let path = path.clone();
        let observed = observed.clone();
        Effect::new(move || {
            observed.borrow_mut().push(path.get()?);
            Ok(())
        })
        .unwrap();
    });

    first.set("First updated".to_string());
    selected.set(Some(Person { name: second }));
    first.set("Old model".to_string());
    second.set("Second updated".to_string());
    path.set("Written".to_string()).unwrap();
    assert_eq!(
        *observed.borrow(),
        [
            "First",
            "First updated",
            "Second",
            "Second updated",
            "Written"
        ]
    );
    assert_eq!(first.get(), "Old model");
    assert_eq!(second.get(), "Written");

    selected.set(None);
    assert!(matches!(
        path.set("Rejected".into()),
        Err(rustolonia::Error::Binding(_))
    ));
    selected.set(Some(Person { name: first }));
    assert_eq!(observed.borrow().last().unwrap(), "Old model");
    lifetime.dispose();
    first.set("After disposal".into());
    assert_eq!(observed.borrow().last().unwrap(), "Old model");
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

#[derive(Clone, Debug, PartialEq)]
struct TodoItem {
    id: u32,
    label: String,
}

#[test]
fn keyed_list_reconciler_reuses_views_and_updates_signals_in_place() {
    let mut reconciler = KeyedListReconciler::<TodoItem, u32, (u32, Signal<TodoItem>)>::new();
    let created_ids = Rc::new(RefCell::new(Vec::new()));
    let applied_lists = Rc::new(RefCell::new(Vec::<Vec<u32>>::new()));

    let initial = vec![
        TodoItem {
            id: 1,
            label: "First".into(),
        },
        TodoItem {
            id: 2,
            label: "Second".into(),
        },
    ];

    let created_for_build = created_ids.clone();
    let applied_for_list = applied_lists.clone();
    let changed = reconciler
        .reconcile(
            initial,
            &mut |item| item.id,
            &mut |sig| {
                let id = sig.get_untracked().id;
                created_for_build.borrow_mut().push(id);
                Ok((id, sig))
            },
            |views| {
                applied_for_list
                    .borrow_mut()
                    .push(views.iter().map(|(id, _)| *id).collect());
                Ok(())
            },
        )
        .expect("initial reconcile");
    assert!(changed);
    assert_eq!(*created_ids.borrow(), vec![1, 2]);
    assert_eq!(*applied_lists.borrow(), vec![vec![1, 2]]);

    // Same keys, updated label on item 2: no new views created and `apply_list` is not called.
    let updated_in_place = vec![
        TodoItem {
            id: 1,
            label: "First".into(),
        },
        TodoItem {
            id: 2,
            label: "Second (edited)".into(),
        },
    ];
    let changed = reconciler
        .reconcile(
            updated_in_place,
            &mut |item| item.id,
            &mut |sig| {
                let id = sig.get_untracked().id;
                created_ids.borrow_mut().push(id);
                Ok((id, sig))
            },
            |views| {
                applied_lists
                    .borrow_mut()
                    .push(views.iter().map(|(id, _)| *id).collect());
                Ok(())
            },
        )
        .expect("in-place reconcile");
    assert!(!changed);
    assert_eq!(
        *created_ids.borrow(),
        vec![1, 2],
        "no new view should be built for existing keys"
    );
    assert_eq!(
        applied_lists.borrow().len(),
        1,
        "apply_list should not run when key order is unchanged"
    );

    // Reorder (2, 1) and add key 3: only key 3 is built, and `apply_list` receives [2, 1, 3].
    let reordered = vec![
        TodoItem {
            id: 2,
            label: "Second (edited)".into(),
        },
        TodoItem {
            id: 1,
            label: "First".into(),
        },
        TodoItem {
            id: 3,
            label: "Third".into(),
        },
    ];
    let mut captured_signals = Vec::new();
    let changed = reconciler
        .reconcile(
            reordered,
            &mut |item| item.id,
            &mut |sig| {
                let id = sig.get_untracked().id;
                created_ids.borrow_mut().push(id);
                Ok((id, sig))
            },
            |views| {
                captured_signals = views.iter().map(|(_, s)| *s).collect();
                applied_lists
                    .borrow_mut()
                    .push(views.iter().map(|(id, _)| *id).collect());
                Ok(())
            },
        )
        .expect("reordered reconcile");
    assert!(changed);
    assert_eq!(*created_ids.borrow(), vec![1, 2, 3]);
    assert_eq!(*applied_lists.borrow(), vec![vec![1, 2], vec![2, 1, 3]]);
    assert_eq!(captured_signals[0].get().label, "Second (edited)");
}

#[test]
fn show_for_control_slot_and_theme_brush_compose_in_rsx() {
    let theme = Signal::new(ThemeVariant::Light);
    let bg = theme_brush(theme, (0xFF, 0xFF, 0xFF), (0x1E, 0x1E, 0x2E));
    assert_eq!(bg.get(), Some(Brush::solid(Color::rgb(0xFF, 0xFF, 0xFF))));
    theme.set(ThemeVariant::Dark);
    assert_eq!(bg.get(), Some(Brush::solid(Color::rgb(0x1E, 0x1E, 0x2E))));

    let visible = Signal::new(true);
    let rows = Signal::new(vec![
        TodoItem {
            id: 1,
            label: "Alpha".into(),
        },
        TodoItem {
            id: 2,
            label: "Beta".into(),
        },
    ]);
    let slot = ControlSlot::new();

    let _window: El<Window> = rsx! {
        Window {
            title: "Reactive Structural Primitives",
            StackPanel {
                background: bg,
                { Show::when_else(visible, || TextBlock::el().text("Shown"), || TextBlock::el().text("Hidden")) }
                { For::each(rows, |item| item.id, |item_sig| TextBlock::el().text(move || item_sig.get().label)) }
                { slot.host() }
            }
        }
    };
    assert!(slot.host_control().is_none());
}

#[test]
fn view_registry_selects_factories_and_reports_missing_or_duplicate_keys() {
    let registry = ViewRegistry::new(|item: &TodoItem| item.id % 2);
    let created = Rc::new(RefCell::new(Vec::new()));
    let created_for_factory = created.clone();
    registry
        .register(0, move |item| {
            created_for_factory
                .borrow_mut()
                .push(item.get_untracked().id);
            TextBlock::el().text(move || item.get().label)
        })
        .unwrap();
    assert!(registry
        .register(0, |_| TextBlock::el().text("duplicate"))
        .is_err());
    let even = Signal::new(TodoItem {
        id: 2,
        label: "Even".into(),
    });
    let odd = Signal::new(TodoItem {
        id: 3,
        label: "Odd".into(),
    });
    let _view = registry.create(even).unwrap();
    assert_eq!(*created.borrow(), vec![2]);
    assert!(registry.create(odd).is_err());

    let rows = Signal::new(vec![even.get()]);
    let _items = ItemsView::from_registry(rows, |item| item.id, registry);
    assert!(matches!(
        ControlSlot::new().clear(),
        Err(rustolonia::Error::ControlSlotNotMounted)
    ));
}

impl MockSink {
    fn set_summary(&self, value: &str) -> Result<()> {
        self.events.borrow_mut().push(format!("summary={value}"));
        Ok(())
    }

    fn set_save_enabled(&self, enabled: bool) -> Result<()> {
        self.events
            .borrow_mut()
            .push(format!("save_enabled={enabled}"));
        Ok(())
    }
}

reactive_view_model! {
    pub struct DerivedDemoViewModel (sink: MockSink) {
        pub name: String = String::from("Initial") => set_name(str_ref),
        pub count: i64 = 0 => set_count(by_val),
    }
    derived {
        pub summary: String = |vm: &Self| format!("{} ({})", vm.name, vm.count) => set_summary(str_ref),
    }
    can_execute {
        pub save_enabled: bool = |vm: &Self| !vm.name.is_empty() && vm.count > 0 => set_save_enabled,
    }
}

#[test]
fn derived_properties_when_any_and_can_execute_recompute_on_state_changes() {
    let mut vm = DerivedDemoViewModel::new();
    assert_eq!(vm.summary(), "Initial (0)");
    assert!(!vm.save_enabled());

    let sink = MockSink::default();
    vm.attach_sink(sink.clone()).expect("attach");
    assert_eq!(
        *sink.events.borrow(),
        vec![
            "name=Initial",
            "count=0",
            "summary=Initial (0)",
            "save_enabled=false",
        ]
    );

    vm.set_count(3).expect("set count to 3");
    assert_eq!(vm.summary(), "Initial (3)");
    assert!(vm.save_enabled());
    assert_eq!(
        *sink.events.borrow(),
        vec![
            "name=Initial",
            "count=0",
            "summary=Initial (0)",
            "save_enabled=false",
            "count=3",
            "summary=Initial (3)",
            "save_enabled=true",
        ]
    );

    // Updating count from 3 to 4 updates `summary`, while `save_enabled` stays `true` (deduplicated!).
    vm.set_count(4).expect("set count to 4");
    assert_eq!(
        *sink.events.borrow(),
        vec![
            "name=Initial",
            "count=0",
            "summary=Initial (0)",
            "save_enabled=false",
            "count=3",
            "summary=Initial (3)",
            "save_enabled=true",
            "count=4",
            "summary=Initial (4)",
        ]
    );

    let combined = when_any!(
        &vm,
        prop!(DerivedDemoViewModel => name),
        prop!(DerivedDemoViewModel => count)
        => |name, count| format!("{name}#{count}")
    );
    assert_eq!(combined, "Initial#4");
}

#[derive(Clone, Debug, PartialEq)]
struct AppThemeContext {
    accent_hex: String,
}

#[test]
fn reactive_scopes_dispose_child_effects_and_resolve_hierarchical_context() {
    let source = Signal::new(10_i32);
    let log = Rc::new(RefCell::new(Vec::new()));

    provide_context(AppThemeContext {
        accent_hex: "#007ACC".into(),
    });
    assert_eq!(
        use_context::<AppThemeContext>().map(|c| c.accent_hex),
        Some("#007ACC".into())
    );

    let log_for_child = log.clone();
    let (_, child_scope) = ReactiveScope::with_new(|| {
        provide_context(AppThemeContext {
            accent_hex: "#FF6600".into(),
        });
        assert_eq!(
            use_context::<AppThemeContext>().map(|c| c.accent_hex),
            Some("#FF6600".into())
        );
        Effect::new(move || {
            log_for_child.borrow_mut().push(source.get());
            Ok(())
        })
        .expect("child effect");
    });

    // Outside the child scope, the root context is still active.
    assert_eq!(
        use_context::<AppThemeContext>().map(|c| c.accent_hex),
        Some("#007ACC".into())
    );
    assert_eq!(*log.borrow(), vec![10]);

    source.set(20);
    assert_eq!(*log.borrow(), vec![10, 20]);

    // Disposing the child scope stops all effects created inside it.
    child_scope.dispose();
    source.set(30);
    assert_eq!(*log.borrow(), vec![10, 20]);
}

#[test]
fn keyed_reconciler_disposes_removed_item_scopes_automatically() {
    let global_tick = Signal::new(0_i32);
    let ticks_seen_by_item_2 = Rc::new(RefCell::new(Vec::new()));
    let mut reconciler = KeyedListReconciler::<TodoItem, u32, u32>::new();

    let ticks_clone = ticks_seen_by_item_2.clone();
    reconciler
        .reconcile(
            vec![
                TodoItem {
                    id: 1,
                    label: "One".into(),
                },
                TodoItem {
                    id: 2,
                    label: "Two".into(),
                },
            ],
            &mut |item| item.id,
            &mut |sig| {
                let id = sig.get_untracked().id;
                if id == 2 {
                    let ticks_inner = ticks_clone.clone();
                    Effect::new(move || {
                        ticks_inner.borrow_mut().push(global_tick.get());
                        Ok(())
                    })?;
                }
                Ok(id)
            },
            |_| Ok(()),
        )
        .expect("initial reconcile");

    assert_eq!(*ticks_seen_by_item_2.borrow(), vec![0]);
    global_tick.set(1);
    assert_eq!(*ticks_seen_by_item_2.borrow(), vec![0, 1]);

    // Remove item 2 from the list: its ReactiveScope is disposed automatically.
    reconciler
        .reconcile(
            vec![TodoItem {
                id: 1,
                label: "One".into(),
            }],
            &mut |item| item.id,
            &mut |sig| Ok(sig.get_untracked().id),
            |_| Ok(()),
        )
        .expect("remove item 2");

    global_tick.set(2);
    assert_eq!(
        *ticks_seen_by_item_2.borrow(),
        vec![0, 1],
        "removed item's effect must not run after reconciliation removes its key"
    );
}

#[test]
fn keyed_reconciler_releases_removed_models_and_scoped_memo_values() {
    let value = Rc::new(String::from("owned by one item"));
    let weak = Rc::downgrade(&value);
    let mut reconciler = KeyedListReconciler::<(u32, Rc<String>), u32, ()>::new();
    reconciler
        .reconcile(
            vec![(1, value)],
            &mut |item| item.0,
            &mut |item| {
                let _label = Memo::new_scoped(move || item.get().1);
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
    assert!(weak.upgrade().is_some());
    reconciler
        .reconcile(Vec::new(), &mut |item| item.0, &mut |_| Ok(()), |_| Ok(()))
        .unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn keyed_reconciler_rejects_duplicate_keys_and_rolls_back_failed_builds() {
    let tick = Signal::new(0);
    let built_ticks = Rc::new(RefCell::new(Vec::new()));
    let mut reconciler = KeyedListReconciler::<TodoItem, u32, Signal<TodoItem>>::new();
    let mut original = None;
    reconciler
        .reconcile(
            vec![TodoItem {
                id: 1,
                label: "Original".into(),
            }],
            &mut |item| item.id,
            &mut Ok,
            |views| {
                original = Some(views[0]);
                Ok(())
            },
        )
        .unwrap();
    let original = original.unwrap();

    let duplicate = reconciler.reconcile(
        vec![
            TodoItem {
                id: 1,
                label: "Changed".into(),
            },
            TodoItem {
                id: 1,
                label: "Duplicate".into(),
            },
        ],
        &mut |item| item.id,
        &mut Ok,
        |_| Ok(()),
    );
    assert!(matches!(
        duplicate,
        Err(rustolonia::Error::DuplicateItemKey)
    ));
    assert_eq!(original.get().label, "Original");

    let ticks = built_ticks.clone();
    let failed = reconciler.reconcile(
        vec![
            TodoItem {
                id: 1,
                label: "Changed".into(),
            },
            TodoItem {
                id: 2,
                label: "New".into(),
            },
            TodoItem {
                id: 3,
                label: "Fails".into(),
            },
        ],
        &mut |item| item.id,
        &mut |item| {
            if item.get_untracked().id == 3 {
                return Err(rustolonia::Error::InvalidAsyncValue);
            }
            let ticks = ticks.clone();
            Effect::new(move || {
                ticks.borrow_mut().push(tick.get());
                Ok(())
            })?;
            Ok(item)
        },
        |_| Ok(()),
    );
    assert!(failed.is_err());
    assert_eq!(reconciler.len(), 1);
    assert_eq!(original.get().label, "Original");
    tick.set(1);
    assert_eq!(*built_ticks.borrow(), vec![0]);

    let failed_apply = reconciler.reconcile(
        vec![TodoItem {
            id: 2,
            label: "New".into(),
        }],
        &mut |item| item.id,
        &mut Ok,
        |_| Err(rustolonia::Error::InvalidAsyncValue),
    );
    assert!(failed_apply.is_err());
    assert_eq!(reconciler.len(), 1);
    assert_eq!(original.get().label, "Original");
}

#[test]
fn item_scopes_created_on_later_updates_follow_the_parent_scope() {
    let rows = Signal::new(vec![TodoItem {
        id: 1,
        label: "One".into(),
    }]);
    let tick = Signal::new(0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let context_seen = Rc::new(RefCell::new(Vec::new()));
    let parent = ReactiveScope::new();
    parent.run(|| {
        provide_context(String::from("parent"));
        let reconciler = Rc::new(RefCell::new(
            KeyedListReconciler::<TodoItem, u32, u32>::new(),
        ));
        let seen = seen.clone();
        let context_seen = context_seen.clone();
        Effect::new(move || {
            context_seen
                .borrow_mut()
                .push(use_context::<String>().unwrap());
            reconciler.borrow_mut().reconcile(
                rows.get(),
                &mut |item| item.id,
                &mut |item| {
                    let id = item.get_untracked().id;
                    if id == 2 {
                        let seen = seen.clone();
                        Effect::new(move || {
                            seen.borrow_mut().push(tick.get());
                            Ok(())
                        })?;
                    }
                    Ok(id)
                },
                |_| Ok(()),
            )?;
            Ok(())
        })
        .unwrap();
    });

    rows.set(vec![
        TodoItem {
            id: 1,
            label: "One".into(),
        },
        TodoItem {
            id: 2,
            label: "Two".into(),
        },
    ]);
    tick.set(1);
    assert_eq!(*seen.borrow(), vec![0, 1]);
    assert_eq!(*context_seen.borrow(), vec!["parent", "parent"]);

    parent.dispose();
    tick.set(2);
    assert_eq!(*seen.borrow(), vec![0, 1]);
}

#[test]
fn safe_reactive_command_tracks_can_execute_and_derived_resource_updates() {
    let scope = ReactiveScope::new();
    scope.run(|| {
        let can_submit = Signal::new(false);
        let executed_with = Arc::new(Mutex::new(Vec::new()));
        let executed_for_cmd = executed_with.clone();

        let cmd = Command::with_can_execute(can_submit, move |param| {
            executed_for_cmd.lock().unwrap().push(param);
            Ok(())
        })
        .expect("create command");

        assert!(!cmd.is_enabled());
        can_submit.set(true);
        assert!(cmd.is_enabled());

        cmd.execute(42_i32).expect("execute command");
        assert_eq!(
            *executed_with.lock().unwrap(),
            vec![rustolonia::Variant::I32(42)]
        );

        let _btn = rsx! {
            Button {
                command: cmd,
                "Submit"
            }
        };

        let query = Signal::new(String::from("rust"));
        let res = Resource::new_derived(
            move || query.get(),
            |q| {
                if q.is_empty() {
                    Err("Query cannot be empty".into())
                } else {
                    Ok(format!("Results for {q}"))
                }
            },
        )
        .expect("derived resource");

        assert_eq!(
            res.get(),
            ResourceState::Ready("Results for rust".to_string())
        );
        query.set(String::new());
        assert_eq!(
            res.get(),
            ResourceState::Error("Query cannot be empty".to_string())
        );
    });
    scope.dispose();
}

#[derive(Clone, Debug, PartialEq)]
struct RecordedCardProps {
    title: String,
    spacing: f64,
    dense: bool,
    subtitle: Option<String>,
    child_count: usize,
}

#[component]
fn SectionCard(
    title: String,
    #[prop(default = 12.0)] spacing: f64,
    #[prop(default)] dense: bool,
    subtitle: Option<String>,
    recorder: Option<Rc<RefCell<Vec<RecordedCardProps>>>>,
    children: Vec<AnyEl>,
) -> El<Border> {
    if let Some(log) = recorder {
        log.borrow_mut().push(RecordedCardProps {
            title: title.clone(),
            spacing,
            dense,
            subtitle: subtitle.clone(),
            child_count: children.len(),
        });
    }
    let padding = if dense { 6.0 } else { 16.0 };
    let mut stack = StackPanel::el()
        .spacing(spacing)
        .child(TextBlock::el().text(title));
    if let Some(sub) = subtitle {
        stack = stack.child(TextBlock::el().text(sub));
    }
    for child in children {
        stack = stack.child(child);
    }
    Border::el().padding(padding).child(stack)
}

#[test]
fn component_macro_supports_required_defaults_optional_props_and_children_in_rsx() {
    let recorded: Rc<RefCell<Vec<RecordedCardProps>>> = Rc::new(RefCell::new(Vec::new()));

    // Direct `.into_el()` invokes the component function immediately without needing a host,
    // letting us assert exact prop resolution and child collection.
    let _default_el = SectionCard::el()
        .title("Overview")
        .recorder(Some(recorded.clone()))
        .child(TextBlock::el().text("Row A"))
        .child("Row B")
        .into_el();

    let _custom_el = SectionCard::el()
        .title("Details")
        .spacing(20.0)
        .dense(true)
        .subtitle("Secondary heading")
        .recorder(Some(recorded.clone()))
        .into_el();

    assert_eq!(
        *recorded.borrow(),
        vec![
            RecordedCardProps {
                title: "Overview".into(),
                spacing: 12.0,
                dense: false,
                subtitle: None,
                child_count: 2,
            },
            RecordedCardProps {
                title: "Details".into(),
                spacing: 20.0,
                dense: true,
                subtitle: Some("Secondary heading".into()),
                child_count: 0,
            },
        ]
    );

    // Also verify full composition inside `rsx!` with shorthand props, defaults, optionals, and nested children.
    let title = "Dashboard";
    let spacing = 18.0;
    let _view: El<Window> = rsx! {
        Window {
            title,
            StackPanel {
                spacing,
                SectionCard {
                    title: "Default Card",
                    TextBlock { "First child" }
                    Button { "Action" }
                }
                SectionCard {
                    title,
                    spacing,
                    dense: true,
                    subtitle: "Explicit optional prop",
                    "Inline text child"
                }
            }
        }
    };
}

#[test]
#[should_panic(expected = "missing required prop 'title'")]
fn component_macro_panics_when_required_prop_is_omitted() {
    let _ = SectionCard::el().spacing(8.0).into_el();
}

#[derive(Clone, Debug, PartialEq, Lens)]
struct ProfileForm {
    username: String,
    age: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct NestedAccount {
    profile: ProfileForm,
    verified: bool,
}

#[test]
fn color_tracks_fmt_and_lens_macros_produce_typed_values_and_writable_paths() {
    const SLATE_BG: Color = color!("#1e293b");
    const TRANSLUCENT_RED: Color = color!("#ff000080");
    const SHORT_CYAN: Color = color!("#0cf");
    const SHORT_ALPHA: Color = color!("#f008");
    const INT_RGB: Color = color!(0x1e293b);
    const INT_RGBA: Color = color!(0xff00_0080);

    assert_eq!(SLATE_BG, Color::rgb(0x1E, 0x29, 0x3B));
    assert_eq!(TRANSLUCENT_RED, Color::rgba(255, 0, 0, 128));
    assert_eq!(SHORT_CYAN, Color::rgb(0x00, 0xCC, 0xFF));
    assert_eq!(SHORT_ALPHA, Color::rgba(0xFF, 0x00, 0x00, 0x88));
    assert_eq!(INT_RGB, SLATE_BG);
    assert_eq!(INT_RGBA, TRANSLUCENT_RED);

    let grid_tracks: GridTracks = tracks![*, Auto@Labels, 120, 2.5*];
    assert_eq!(
        grid_tracks,
        GridTracks(vec![
            GridTrack::star(1.0),
            GridTrack::auto().shared_group("Labels"),
            GridTrack::px(120.0),
            GridTrack::star(2.5),
        ])
    );
    assert_eq!(grid_tracks.to_string(), "*,Auto@Labels,120,2.5*");

    let count = Signal::new(0_i32);
    let formatted = fmt!("Count: {}", count.get());
    assert_eq!(formatted(), "Count: 0");
    count.set(9);
    assert_eq!(formatted(), "Count: 9");

    // `#[derive(Lens)]` generates `lens_<field>` writable `PropertyPath`s.
    let form = Signal::new(ProfileForm {
        username: "ada".into(),
        age: 36,
    });
    let username_lens = ProfileForm::lens_username(form);
    let age_lens = ProfileForm::lens_age(form);
    let observed_names = Rc::new(RefCell::new(Vec::new()));
    let observed_for_effect = observed_names.clone();
    let username_for_effect = username_lens.clone();
    let lens_effect = Effect::new(move || {
        observed_for_effect
            .borrow_mut()
            .push(username_for_effect.get()?);
        Ok(())
    })
    .unwrap();

    assert_eq!(*observed_names.borrow(), vec!["ada"]);
    assert_eq!(age_lens.get().unwrap(), 36);

    username_lens.set("grace".into()).unwrap();
    assert_eq!(*observed_names.borrow(), vec!["ada", "grace"]);
    age_lens.set(40).unwrap();
    assert_eq!(
        form.get(),
        ProfileForm {
            username: "grace".into(),
            age: 40,
        }
    );
    lens_effect.dispose();

    // `lens!(sig => ...)` (read-only) and `lens!(mut sig => ...)` (writable, including nested fields).
    let account = Signal::new(NestedAccount {
        profile: ProfileForm {
            username: "linus".into(),
            age: 21,
        },
        verified: false,
    });
    let ro_verified = lens!(account => verified);
    assert!(!ro_verified.get().unwrap());
    assert!(matches!(
        ro_verified.set(true),
        Err(Error::ReadOnlyBindingPath)
    ));

    let mut_nested_name = lens!(mut account => profile.username);
    let mut_verified = lens!(mut account => verified);
    assert_eq!(mut_nested_name.get().unwrap(), "linus");
    mut_nested_name.set("margaret".into()).unwrap();
    mut_verified.set(true).unwrap();
    assert_eq!(account.get().profile.username, "margaret");
    assert!(account.get().verified);
}

#[test]
fn rsx_supports_shorthand_props_reactive_show_else_for_each_and_fmt_children() {
    let title = "Shorthand & Reactive Control Flow";
    let spacing = 10.0;
    let count = Signal::new(4_i32);
    let is_expanded = Signal::new(true);
    let items = Signal::new(vec![
        TodoItem {
            id: 10,
            label: "Write tests".into(),
        },
        TodoItem {
            id: 20,
            label: "Ship release".into(),
        },
    ]);

    let _window: El<Window> = rsx! {
        Window {
            title,
            Grid {
                column_definitions: tracks![*, Auto@Labels, 120, 2.5*],
                StackPanel {
                    spacing,
                    background: color!("#1e293b"),
                    fmt!("Count: {}", count.get())
                    show is_expanded {
                        TextBlock {
                            foreground: color!("#ff000080"),
                            text: fmt!("Expanded ({})", count.get()),
                        }
                    } else {
                        TextBlock { "Collapsed" }
                    }
                    show when is_expanded {
                        TextBlock { "Optional banner" }
                    }
                    StackPanel {
                        for each item in items key |x| x.id {
                            TextBlock {
                                text: move || item.get().label,
                            }
                        }
                    }
                }
            }
        }
    };
}

#[test]
fn non_send_captures_and_with_ui_error_handler_work_across_callbacks_commands_and_effects() {
    // `Rc<RefCell<...>>` is `!Send` and `!Sync`.
    let local_log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    // 1. `El::on_click` accepts `!Send` captures AND fallible `move |_| -> Result<()>` closures.
    let log_for_click = local_log.clone();
    let _fallible_button: El<Button> =
        Button::el()
            .content("Trigger")
            .on_click(move |_| -> Result<()> {
                log_for_click.borrow_mut().push("on_click".into());
                Err(Error::Binding("fallible on_click error".into()))
            });

    let log_for_rsx_click = local_log.clone();
    let _rsx_button: El<Button> = rsx! {
        Button {
            on_click: move |_| -> Result<()> {
                log_for_rsx_click.borrow_mut().push("rsx_on_click".into());
                Err(Error::Binding("rsx on_click error".into()))
            },
            "RSX Trigger"
        }
    };

    // 2. `PropertyPath::writable` accepts `!Send` captures in both reader and writer.
    let backing_text: Rc<RefCell<String>> = Rc::new(RefCell::new("hello".into()));
    let backing_for_read = backing_text.clone();
    let backing_for_write = backing_text.clone();
    let log_for_path = local_log.clone();
    let writable_path = PropertyPath::writable(
        move || Ok(backing_for_read.borrow().clone()),
        move |next: String| {
            log_for_path.borrow_mut().push(format!("path={next}"));
            if next.is_empty() {
                return Err(Error::Binding("path rejected empty string".into()));
            }
            *backing_for_write.borrow_mut() = next;
            Ok(())
        },
    );
    assert_eq!(writable_path.get().unwrap(), "hello");
    writable_path.set("world".into()).unwrap();
    assert_eq!(&*backing_text.borrow(), "world");

    // 3. `Command::new` accepts `!Send` captures and routes `Err(...)` to `with_ui_error_handler`.
    let log_for_cmd = local_log.clone();
    let fallible_cmd = Command::new(move |param| -> Result<()> {
        log_for_cmd.borrow_mut().push(format!("cmd={param:?}"));
        if param == Variant::I32(-1) {
            return Err(Error::Binding("negative command parameter".into()));
        }
        Ok(())
    });

    // 4. `Effect::new` updates route `Err(...)` to `with_ui_error_handler`.
    let step = Signal::new(0_i32);
    let log_for_effect = local_log.clone();
    let effect = Effect::new(move || -> Result<()> {
        let current = step.get();
        log_for_effect
            .borrow_mut()
            .push(format!("effect={current}"));
        if current < 0 {
            return Err(Error::Binding("negative effect state".into()));
        }
        Ok(())
    })
    .expect("initial effect succeeds");

    let captured_errors: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let errors_sink = captured_errors.clone();

    with_ui_error_handler(
        move |context, err| {
            errors_sink
                .lock()
                .unwrap()
                .push((context.to_string(), err.to_string()));
        },
        || {
            fallible_cmd.execute(1_i32).expect("valid command succeeds");
            assert!(fallible_cmd.execute(-1_i32).is_err());
            step.set(-5);
        },
    );

    effect.dispose();

    assert_eq!(
        *local_log.borrow(),
        vec![
            "path=world",
            "effect=0",
            "cmd=I32(1)",
            "cmd=I32(-1)",
            "effect=-5",
        ]
    );

    let errors = captured_errors.lock().unwrap().clone();
    assert_eq!(
        errors.len(),
        2,
        "expected command and effect errors: {errors:?}"
    );
    assert_eq!(errors[0].0, "command callback failed");
    assert!(
        errors[0].1.contains("negative command parameter"),
        "{errors:?}"
    );
    assert_eq!(errors[1].0, "reactive effect failed");
    assert!(errors[1].1.contains("negative effect state"), "{errors:?}");
}

#[test]
fn stylesheet_style_select_shortcut_and_box_shadow_construct_and_serialize() {
    let drop_shadow = BoxShadow::new(0.0, 4.0, 16.0, color!("#00000040")).spread(2.0);
    assert_eq!(drop_shadow.to_string(), "0 4 16 2 #40000000");

    let inner_shadow = BoxShadow::new(1.0, 2.0, 4.0, Color::rgb(0x10, 0x20, 0x30)).inset(true);
    assert_eq!(inner_shadow.to_string(), "inset 1 2 4 0 #FF102030");

    assert_eq!(BoxShadows::none().to_string(), "none");
    let layered = BoxShadows::from([drop_shadow, inner_shadow]);
    assert_eq!(
        layered.to_string(),
        "0 4 16 2 #40000000, inset 1 2 4 0 #FF102030"
    );

    let open_shortcut = Shortcut::primary(Key::O);
    let palette_shortcut = Shortcut::primary_shift(Key::P);
    let custom_shortcut = Shortcut::new(KeyModifiers::ALT | KeyModifiers::SHIFT, Key::F5);
    let bare_shortcut = Shortcut::key(Key::Escape);

    if cfg!(any(target_os = "macos", target_os = "ios")) {
        assert_eq!(open_shortcut.to_gesture_string(), "Cmd+O");
        assert_eq!(palette_shortcut.to_gesture_string(), "Cmd+Shift+P");
    } else {
        assert_eq!(open_shortcut.to_gesture_string(), "Ctrl+O");
        assert_eq!(palette_shortcut.to_gesture_string(), "Ctrl+Shift+P");
    }
    assert_eq!(custom_shortcut.to_gesture_string(), "Alt+Shift+F5");
    assert_eq!(bare_shortcut.to_gesture_string(), "Escape");
    let gesture_opt: Option<String> = open_shortcut.into_value();
    assert_eq!(gesture_opt, Some(open_shortcut.to_string()));

    let linear = LinearGradientBrush::new(
        vec![
            GradientStop::new(0.0, color!("#2563eb")),
            GradientStop::new(1.0, color!("#7c3aed")),
        ],
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
        RelativePoint::new(1.0, 1.0, RelativeUnit::Relative),
    );
    let radial = RadialGradientBrush {
        opacity: 0.9,
        spread_method: SpreadMethod::Reflect,
        stops: vec![GradientStop::new(0.0, Color::WHITE)],
        center: RelativePoint::new(0.5, 0.5, RelativeUnit::Relative),
        gradient_origin: RelativePoint::new(0.5, 0.5, RelativeUnit::Relative),
        radius_x: RelativeScalar::new(0.5, RelativeUnit::Relative),
        radius_y: RelativeScalar::new(0.5, RelativeUnit::Relative),
    };

    let hover_rule = Style::select("Button.primary.cta:pointerover")
        .background(color!("#1d4ed8"))
        .foreground(Color::WHITE)
        .border_brush(linear)
        .border_thickness(1.0)
        .corner_radius(6.0)
        .padding((16.0, 8.0))
        .font_size(15.0)
        .font_weight(FontWeight::DemiBold)
        .opacity(0.95)
        .transition_background(Duration::from_millis(180))
        .transition_opacity(Duration::from_millis(120));

    let card_rule = Style::control("Border")
        .class("card")
        .focused()
        .background(radial)
        .box_shadow(layered);

    assert!(StyleSheet::new().is_empty());
    let sheet = StyleSheet::new().rule(hover_rule).rule(card_rule);
    assert!(!sheet.is_empty());

    let json = sheet.to_json();
    assert!(json.starts_with("{\"rules\":["), "{json}");
    assert!(json.contains("\"target\":\"Button\""), "{json}");
    assert!(json.contains("\"classes\":[\"primary\",\"cta\"]"), "{json}");
    assert!(json.contains("\"pseudos\":[\":pointerover\"]"), "{json}");
    assert!(json.contains("\"kind\":\"solid\""), "{json}");
    assert!(json.contains("\"kind\":\"linear\""), "{json}");
    assert!(json.contains("\"kind\":\"radial\""), "{json}");
    assert!(
        json.contains("\"prop\":\"cornerRadius\",\"value\":[6,6,6,6]"),
        "{json}"
    );
    assert!(
        json.contains("\"prop\":\"padding\",\"value\":[16,8,16,8]"),
        "{json}"
    );
    assert!(
        json.contains("\"transitions\":[{\"prop\":\"background\",\"durationMs\":180},{\"prop\":\"opacity\",\"durationMs\":120}]"),
        "{json}"
    );
    assert!(json.contains("\"target\":\"Border\""), "{json}");
    assert!(json.contains("\"pseudos\":[\":focus\"]"), "{json}");
    assert!(json.contains("\"prop\":\"boxShadow\""), "{json}");

    // Verify `El::styles` attaches a `StyleSheet` declaratively.
    let _styled_btn: El<Button> = Button::el()
        .class("primary cta")
        .styles(sheet)
        .content("Save");
}

#[test]
fn typed_dates_times_decimals_flags_and_safe_callback_wrappers_work_purely_in_rust() {
    // CivilDate
    let leap = CivilDate::new(2024, 2, 29).unwrap();
    assert_eq!((leap.year(), leap.month(), leap.day()), (2024, 2, 29));
    assert_eq!(leap.to_string(), "2024-02-29");
    assert_eq!("2024-02-29".parse::<CivilDate>().unwrap(), leap);
    assert!(CivilDate::new(2025, 2, 29).is_err());
    assert!("03/09/2026".parse::<CivilDate>().is_err());

    // CivilTime
    let time = CivilTime::hm(17, 4).unwrap();
    assert_eq!((time.hour(), time.minute(), time.second()), (17, 4, 0));
    assert_eq!(time.to_string(), "17:04:00");
    assert_eq!("17:04".parse::<CivilTime>().unwrap(), time);
    assert_eq!("17:04:00".parse::<CivilTime>().unwrap(), time);
    assert!(CivilTime::new(24, 0, 0).is_err());
    assert!("PT8H15M".parse::<CivilTime>().is_err());

    // Decimal
    let d1: Decimal = "12.50".parse().unwrap();
    let d2 = Decimal::new(125, 1).unwrap();
    assert_eq!(d1, d2);
    assert_eq!(d1.normalize().to_string(), "12.5");
    assert_eq!(Decimal::from(-42_i32).to_string(), "-42");
    assert!(Decimal::try_from(f64::NAN).is_err());

    // Typed `[Flags]` structs
    let sel = SelectionMode::MULTIPLE | SelectionMode::ALWAYS_SELECTED;
    assert!(sel.contains(SelectionMode::MULTIPLE));
    assert!(sel.contains(SelectionMode::ALWAYS_SELECTED));
    let mods = KeyModifiers::ALT | KeyModifiers::SHIFT;
    assert!(mods.contains(KeyModifiers::ALT));
    assert!(!mods.contains(KeyModifiers::CONTROL));
    let adj =
        PopupPositionerConstraintAdjustment::FLIP_X | PopupPositionerConstraintAdjustment::SLIDE_Y;
    assert!(adj.contains(PopupPositionerConstraintAdjustment::FLIP_X));

    // Safe event args structs
    let key_args = ControlKeyDownEventArgs {
        key: Key::Return,
        physical_key: PhysicalKey::Enter,
        key_modifiers: KeyModifiers::CONTROL,
        key_symbol: Some("\r".into()),
        handled: false,
    };
    assert_eq!(key_args.key, Key::Return);
    let sel_args = SelectingItemsControlSelectionChangedArgs {
        added_items: vec![Variant::from("Alpha")],
        removed_items: vec![],
    };
    assert_eq!(sel_args.added_items.len(), 1);

    // Safe callback wrappers with `!Send` state (`Rc<RefCell<...>>`)
    let counter = Rc::new(RefCell::new(0_usize));
    let c1 = counter.clone();
    let item_filter = ItemFilter::new(move |search, item| {
        *c1.borrow_mut() += 1;
        matches!(item, Variant::Utf16(s) if s.contains(search))
    });
    let text_filter = TextFilter::new(|search, text| text.starts_with(search));
    let item_selector = ItemSelector::new(|_search, item| format!("{item:?}"));
    let text_selector = TextSelector::new(|_search, text| text.to_uppercase());
    let async_pop = AsyncPopulator::new(|_search, req| req.complete_empty());
    let placement_cb =
        PopupPlacementCallback::new(|input: PopupPlacementInput| PopupPlacementOutput {
            offset: Vector::new(0.0, input.anchor_rect.height),
            anchor: PopupAnchor::TOP_LEFT,
            gravity: PopupGravity::BOTTOM_RIGHT,
            constraint_adjustment: PopupPositionerConstraintAdjustment::FLIP_X,
        });
    let c2 = counter.clone();
    let _notification = Notification::new("Build succeeded", "All checks green")
        .kind(NotificationType::Success)
        .expiration(Duration::from_secs(3))
        .on_click(move || -> Result<()> {
            *c2.borrow_mut() += 1;
            Ok(())
        })
        .on_close(|| {});

    let _autocomplete: El<AutoCompleteBox> = AutoCompleteBox::el()
        .item_filter(item_filter)
        .text_filter(text_filter)
        .item_selector(item_selector)
        .text_selector(text_selector)
        .async_populator(async_pop);
    let _popup: El<Popup> = Popup::el().custom_popup_placement_callback(placement_cb);
}
