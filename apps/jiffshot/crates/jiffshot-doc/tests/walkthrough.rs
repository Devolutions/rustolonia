//! Every tool in sequence on one document, as the editor's UI self-test
//! drives them, with timings.

use jiffshot_doc::editor::{Editor, Modifiers, Tool};
use jiffshot_doc::{Document, Pt, Rgba};
use std::time::Instant;

const NONE: Modifiers = Modifiers { shift: false, ctrl: false, alt: false };

fn drag(e: &mut Editor, tool: Tool, a: (f32, f32), b: (f32, f32), steps: usize) {
    let started = Instant::now();
    e.set_tool(tool);
    e.pointer_down(Pt::new(a.0, a.1), NONE, 6.0);
    e.take_damage();
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        e.pointer_move(Pt::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), NONE);
        e.take_damage();
    }
    e.pointer_up(Pt::new(b.0, b.1), NONE);
    e.take_damage();
    println!("{tool:?}: {:.1} ms, {} objects", started.elapsed().as_secs_f64() * 1000.0, e.document().objects.len());
}

#[test]
fn every_tool_in_sequence() {
    let (w, h) = (1600.0, 1000.0);
    let mut e = Editor::new(Document::blank(1600, 1000, Rgba::rgb(240, 240, 240)));
    e.take_damage();
    let p = |x: f32, y: f32| (w * x, h * y);
    drag(&mut e, Tool::Arrow, p(0.08, 0.12), p(0.30, 0.30), 12);
    drag(&mut e, Tool::Shape, p(0.35, 0.10), p(0.55, 0.30), 12);
    drag(&mut e, Tool::Text, p(0.60, 0.08), p(0.92, 0.18), 12);
    e.set_selected_text("Jiffshot text");
    drag(&mut e, Tool::Callout, p(0.06, 0.42), p(0.30, 0.58), 12);
    e.set_selected_text("A callout with a tail");
    for x in [0.40, 0.47, 0.54] {
        drag(&mut e, Tool::Step, p(x, 0.45), p(x, 0.45), 0);
    }
    drag(&mut e, Tool::Pen, p(0.62, 0.40), p(0.90, 0.55), 30);
    drag(&mut e, Tool::Highlight, p(0.60, 0.62), p(0.90, 0.70), 12);
    drag(&mut e, Tool::Blur, p(0.06, 0.68), p(0.26, 0.90), 12);
    drag(&mut e, Tool::Magnify, p(0.32, 0.66), p(0.46, 0.88), 12);
    drag(&mut e, Tool::Stamp, p(0.52, 0.78), p(0.52, 0.78), 0);
    drag(&mut e, Tool::Line, p(0.60, 0.80), p(0.90, 0.92), 12);
    drag(&mut e, Tool::Spotlight, p(0.33, 0.33), p(0.50, 0.50), 12);
    drag(&mut e, Tool::Select, p(0.19, 0.21), p(0.24, 0.26), 12);
    drag(&mut e, Tool::Crop, p(0.1, 0.1), p(0.6, 0.6), 12);
    assert!(e.document().objects.len() >= 12);
    assert_eq!(e.document().size(), (800, 500));
}

#[test]
fn wand_selection_and_eraser() {
    use jiffshot_doc::editor::BackgroundFill;
    use jiffshot_doc::raster;
    let mut bg = raster::solid(400, 300, Rgba::WHITE).unwrap();
    for y in 100..160 {
        for x in 100..220 {
            raster::set_pixel(&mut bg, x, y, Rgba::rgb(30, 110, 220));
        }
    }
    let mut e = Editor::new(Document::new(bg));
    e.set_tool(Tool::MagicWand);
    e.pointer_down(Pt::new(150.0, 130.0), NONE, 6.0);
    e.pointer_up(Pt::new(150.0, 130.0), NONE);
    let lifted = e.selected().expect("the blue block is now an object").rect().unwrap();
    assert_eq!((lifted.x0, lifted.y0, lifted.width(), lifted.height()), (100.0, 100.0, 120.0, 60.0));
    assert_eq!(raster::pixel(e.document().background(), 150, 130), Rgba::WHITE, "auto-filled");

    e.background_fill = BackgroundFill::Transparent;
    e.set_tool(Tool::Selection);
    e.set_variant("Ellipse");
    drag(&mut e, Tool::Selection, (250.0, 50.0), (350.0, 150.0), 4);
    assert_eq!(raster::pixel(e.document().background(), 300, 100).a, 0, "transparent hole");
    assert_eq!(raster::pixel(e.document().background(), 252, 52).a, 255, "outside the ellipse stays");

    drag(&mut e, Tool::Eraser, (20.0, 250.0), (200.0, 250.0), 6);
    assert_eq!(raster::pixel(e.document().background(), 100, 250).a, 0);
    assert!(e.undo());
    assert_eq!(raster::pixel(e.document().background(), 100, 250).a, 255);
}

#[test]
fn canvas_resize_by_dragging_and_expand_to_fit() {
    let mut e = Editor::new(Document::blank(400, 300, Rgba::WHITE));
    e.set_tool(Tool::Select);
    drag(&mut e, Tool::Select, (400.0, 300.0), (500.0, 340.0), 4);
    assert_eq!(e.document().size(), (500, 340));
    drag(&mut e, Tool::Shape, (450.0, 300.0), (620.0, 420.0), 4);
    e.select(None);
    assert!(e.expand_to_fit());
    let (w, h) = e.document().size();
    assert!(w >= 620 && h >= 420, "{w}x{h}");
}

#[test]
fn select_several_group_align() {
    let mut e = Editor::new(Document::blank(800, 600, Rgba::WHITE));
    for (x, y) in [(50.0, 50.0), (300.0, 120.0), (500.0, 300.0)] {
        drag(&mut e, Tool::Shape, (x, y), (x + 80.0, y + 60.0), 4);
    }
    // Box select the first two with the Select tool.
    drag(&mut e, Tool::Select, (20.0, 20.0), (420.0, 220.0), 4);
    assert_eq!(e.selected_ids().len(), 2);
    let shift = Modifiers { shift: true, ctrl: false, alt: false };
    e.pointer_down(Pt::new(500.0, 330.0), shift, 6.0);
    e.pointer_up(Pt::new(500.0, 330.0), shift);
    assert_eq!(e.selected_ids().len(), 3, "Shift-click adds");
    e.align_selected("left");
    let lefts: Vec<f32> = e.document().objects.iter().map(|o| o.rect().unwrap().x0).collect();
    assert!(lefts.iter().all(|x| (*x - 50.0).abs() < 1e-3), "{lefts:?}");
    let g = e.group_selected().unwrap();
    assert_eq!(e.document().objects.len(), 1);
    // Moving the group moves all three.
    drag(&mut e, Tool::Select, (50.0, 80.0), (150.0, 80.0), 4);
    assert_eq!(e.document().get(g).unwrap().rect().unwrap().x0, 150.0);
    assert!(e.ungroup_selected());
    assert_eq!(e.document().objects.len(), 3);
    assert_eq!(e.selected_ids().len(), 3);
    e.delete_selected();
    assert!(e.document().objects.is_empty());
    assert!(e.undo());
    assert_eq!(e.document().objects.len(), 3);
}

#[test]
fn polygon_global_fill_cut_out_edges_and_step_formats() {
    use jiffshot_doc::object::Kind;
    use jiffshot_doc::raster;
    let mut e = Editor::new(Document::blank(400, 300, Rgba::WHITE));
    e.set_tool(Tool::Shape);
    e.set_variant("Polygon");
    for (x, y) in [(50.0, 50.0), (200.0, 60.0), (120.0, 200.0)] {
        e.pointer_down(Pt::new(x, y), NONE, 6.0);
        e.pointer_move(Pt::new(x + 5.0, y + 5.0), NONE);
        e.pointer_up(Pt::new(x, y), NONE);
    }
    e.pointer_down(Pt::new(51.0, 51.0), NONE, 6.0);
    e.pointer_up(Pt::new(51.0, 51.0), NONE);
    let Kind::Polygon { points } = &e.document().objects[0].kind else { panic!("a polygon") };
    assert_eq!(points.len(), 3, "{points:?}");

    let mut bg = raster::solid(100, 100, Rgba::WHITE).unwrap();
    for x in [10, 80] {
        raster::set_pixel(&mut bg, x, 10, Rgba::BLACK);
    }
    let mut e = Editor::new(Document::new(bg));
    e.set_tool(Tool::Fill);
    e.set_variant("Global");
    e.set_tool_style(Tool::Fill, jiffshot_doc::object::Style { fill: Rgba::RED, ..Default::default() });
    e.pointer_down(Pt::new(10.0, 10.0), NONE, 6.0);
    e.pointer_up(Pt::new(10.0, 10.0), NONE);
    assert_eq!(raster::pixel(e.document().background(), 80, 10), Rgba::RED, "every black pixel");

    let mut d = Document::blank(200, 200, Rgba::WHITE);
    assert!(jiffshot_doc::effects::cut_out_with(&mut d, jiffshot_doc::IRect::new(0, 50, 200, 60), false, jiffshot_doc::effects::Edge::Sawtooth));
    assert_eq!(d.height(), 200 - 60 + 14);
    assert!((0..200).any(|x| raster::pixel(d.background(), x, 57).a == 0), "a gap in the seam");

    let mut e = Editor::new(Document::blank(300, 100, Rgba::WHITE));
    e.set_tool(Tool::Step);
    e.set_variant("Roman");
    for x in [30.0, 80.0, 130.0, 180.0] {
        e.select(None);
        e.pointer_down(Pt::new(x, 50.0), NONE, 6.0);
        e.pointer_up(Pt::new(x, 50.0), NONE);
    }
    let labels: Vec<_> = e.document().objects.iter().filter_map(|o| o.text()).collect();
    assert_eq!(labels, ["i", "ii", "iii", "iv"]);
}

#[test]
fn template_drop_zones_fill_swap_fit_and_pan() {
    use jiffshot_doc::combine::{template_with, Options};
    use jiffshot_doc::object::Kind;
    use jiffshot_doc::raster;
    let red = raster::solid(400, 200, Rgba::RED).unwrap();
    let blue = raster::solid(200, 400, Rgba::BLUE).unwrap();
    let doc = template_with("Before and after", &[], &Options::default()).unwrap();
    let mut e = Editor::new(doc);
    let zones: Vec<(u64, jiffshot_doc::Box2)> =
        e.document().objects.iter().filter(|o| o.zone.is_some()).map(|o| (o.id, match &o.kind { Kind::Image { rect, .. } => *rect, _ => unreachable!() })).collect();
    assert_eq!(zones.len(), 2);
    let image_of = |e: &Editor, id: u64| match &e.document().get(id).unwrap().kind {
        Kind::Image { image, .. } => image.pixmap.as_ref().map(|p| (p.width(), p.height())),
        _ => None,
    };
    // Dropping onto a zone fills it; dropping elsewhere adds an image.
    let (a, ra) = zones[0];
    let (b, rb) = zones[1];
    e.drop_image(ra.center(), red.clone());
    e.drop_image(rb.center(), blue.clone());
    assert_eq!((image_of(&e, a), image_of(&e, b)), (Some((400, 200)), Some((200, 400))));
    let before = e.document().objects.len();
    e.drop_image(Pt::new(5.0, 5.0), red.clone());
    assert_eq!(e.document().objects.len(), before + 1);
    // Swap the two zones' pictures, then undo.
    e.set_selection(vec![a, b]);
    assert!(e.swap_zones());
    assert_eq!(image_of(&e, a), Some((200, 400)));
    e.undo();
    assert_eq!(image_of(&e, a), Some((400, 200)));
    // Dragging inside a selected, filled zone pans its picture.
    e.set_tool(Tool::Select);
    e.select(Some(b));
    let c = rb.center();
    e.pointer_down(c, NONE, 6.0);
    e.pointer_move(Pt::new(c.x, c.y + 60.0), NONE);
    e.pointer_up(Pt::new(c.x, c.y + 60.0), NONE);
    let pan = e.document().get(b).unwrap().zone.unwrap().pan;
    assert!(pan.y > 50.0 && pan.x == 0.0, "panned down: {pan:?}");
    let Kind::Image { rect, .. } = &e.document().get(b).unwrap().kind else { unreachable!() };
    assert_eq!(*rect, rb);
    // Fit shows the whole picture: blue is narrower than the zone.
    e.select(Some(b));
    e.set_zone_fit(true);
    let flat = e.document().flatten();
    let edge = raster::pixel(&flat, (rb.x0 + 3.0) as i32, rb.center().y as i32);
    assert!(edge.b < 200 || edge.r > 100, "letterboxed at the edge: {edge:?}");
    // Saved and loaded, with an emptied zone.
    e.select(Some(a));
    e.empty_zone();
    let dir = std::env::temp_dir().join(format!("jiffshot-zones-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.jsd");
    jiffshot_doc::io::save_document(e.document(), &path).unwrap();
    let back = jiffshot_doc::io::load_document(&path).unwrap();
    let z: Vec<_> = back.objects.iter().filter(|o| o.zone.is_some()).collect();
    assert_eq!(z.len(), 2);
    assert!(matches!(&z[0].kind, Kind::Image { image, .. } if image.pixmap.is_none()));
    assert!(z[1].zone.unwrap().fit);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pen_smoothing_callout_tails_and_spotlight_magnify() {
    use jiffshot_doc::object::Kind;
    let mut e = Editor::new(Document::blank(800, 600, Rgba::WHITE));
    // A jittery stroke comes out smoother with the default smoothing.
    e.set_tool(Tool::Pen);
    e.pointer_down(Pt::new(100.0, 300.0), NONE, 6.0);
    for i in 1..=60 {
        let jitter = if i % 2 == 0 { 6.0 } else { -6.0 };
        e.pointer_move(Pt::new(100.0 + i as f32 * 8.0, 300.0 + jitter), NONE);
    }
    e.pointer_up(Pt::new(588.0, 300.0), NONE);
    let pen = e.document().objects.last().unwrap();
    let Kind::Pen { points } = &pen.kind else { panic!("{:?}", pen.kind) };
    let wobble = points.iter().map(|p| (p.y - 300.0).abs()).fold(0.0f32, f32::max);
    assert!(wobble < 3.0, "smoothed: {wobble}");
    assert_eq!(points.first().unwrap().x, 100.0, "ends kept");
    // A callout with three tails, each with a handle; undo removes one.
    e.set_tool(Tool::Callout);
    e.pointer_down(Pt::new(300.0, 80.0), NONE, 6.0);
    e.pointer_move(Pt::new(500.0, 160.0), NONE);
    e.pointer_up(Pt::new(500.0, 160.0), NONE);
    assert!(e.callout_tail(true) && e.callout_tail(true));
    let c = e.selected().unwrap();
    let Kind::Callout { more_tails, .. } = &c.kind else { panic!() };
    assert_eq!(more_tails.len(), 2);
    assert_eq!(c.handles().iter().filter(|(h, _)| *h >= jiffshot_doc::object::H_MORE_TAILS && *h < jiffshot_doc::object::H_VERTEX).count(), 2);
    e.undo();
    let Kind::Callout { more_tails, .. } = &e.selected().unwrap().kind else { panic!() };
    assert_eq!(more_tails.len(), 1);
    // Spotlight & magnify on a drawn rectangle: it becomes a framed,
    // enlarged picture over a dimmed image.
    drag(&mut e, Tool::Shape, (50.0, 450.0), (150.0, 530.0), 4);
    let before = e.document().objects.len();
    e.spotlight_magnify(1.6);
    let objects = &e.document().objects;
    assert_eq!(objects.len(), before + 1, "the rectangle is replaced by a spotlight and an image");
    assert!(matches!(objects[objects.len() - 2].kind, Kind::Spotlight { .. }));
    let Kind::Image { rect, .. } = &objects.last().unwrap().kind else { panic!() };
    assert!((rect.width() - 160.0).abs() < 1.0 && (rect.height() - 128.0).abs() < 1.0, "{rect:?}");
}
