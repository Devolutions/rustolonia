//! Tests for hierarchy upcasting (`Upcast`, `Deref`), ergonomic value conversions
//! (`From`/`Into`, `IntoValue`, `IntoProp`), fine-grained reactivity (`Signal`,
//! `Memo`, `Effect`, `NodeRef`, `TwoWay`), `El<T>`, `rsx!`, and `reactive_view_model!`.

use rustolonia::{
    prop, provide_context, reactive_view_model, rsx, theme_brush, use_context, when_any, AsControl,
    AvaloniaObject, Border, Brush, Button, CheckBox, Color, Command, ContentControl, Control,
    ControlSlot, CornerRadius, Dock, Downcast, Effect, El, For, IntoProp, IntoValue,
    KeyedListReconciler, ListBox, ListBoxItem, Memo, NodeRef, NonOwningRef, Orientation, Paint,
    PixelPoint, PlacementMode, Point, PropBinding, ReactiveScope, Rect, Resource, ResourceState,
    Result, Show, Signal, Size, Slider, StackPanel, StyledElement, TemplatedControl, TextBlock,
    TextBox, ThemeVariant, Thickness, ToggleSwitch, Upcast, Vector, Window,
};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
fn safe_reactive_command_tracks_can_execute_and_derived_resource_updates() {
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
}
