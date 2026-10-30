//! Renders one of each object style to a PNG for checking by eye:
//! `cargo run -p jiffshot-doc --example gallery -- OUT.png`.

use jiffshot_doc::object::{Cap, Kind, ShapeKind, Style, VAlign};
use jiffshot_doc::text::Align;
use jiffshot_doc::{io, Box2, Document, Pt, Rgba};

fn main() {
    let out = std::env::args().nth(1).expect("usage: gallery OUT.png");
    let mut d = Document::blank(1200, 800, Rgba::rgb(0xF4, 0xF5, 0xF7));
    let blue = Rgba::rgb(0x1E, 0x6F, 0xD9);
    for (i, (_, cap)) in jiffshot_doc::object::CAPS.iter().enumerate() {
        let y = 40.0 + i as f32 * 42.0;
        let style = Style { stroke: Rgba::RED, width: 6.0, end_cap: *cap, start_cap: if *cap == Cap::Taper { Cap::Taper } else { Cap::None }, ..Style::default() };
        d.add(Kind::Line { a: Pt::new(40.0, y), b: Pt::new(300.0, y), control: None }, style);
    }
    let big = Style { stroke: blue, width: 6.0, end_cap: Cap::Arrow, head_size: 1.8, shadow: true, shadow_distance: 8.0, shadow_blur: 6.0, ..Style::default() };
    d.add(Kind::Line { a: Pt::new(40.0, 360.0), b: Pt::new(300.0, 420.0), control: Some(Pt::new(170.0, 300.0)) }, big);
    let grad = Style { fill: blue, fill2: Rgba::rgb(0x8E, 0x24, 0xAA), stroke: Rgba::TRANSPARENT, width: 0.0, shadow: true, ..Style::default() };
    d.add(Kind::Shape { rect: Box2::from_xywh(360.0, 40.0, 220.0, 140.0), shape: ShapeKind::Rounded }, grad.clone());
    d.add(Kind::Shape { rect: Box2::from_xywh(620.0, 40.0, 160.0, 140.0), shape: ShapeKind::Ellipse }, Style { gradient_radial: true, fill: Rgba::YELLOW, fill2: Rgba::RED, ..grad.clone() });
    d.add(Kind::Polygon { points: vec![Pt::new(840.0, 180.0), Pt::new(900.0, 40.0), Pt::new(1000.0, 90.0), Pt::new(1140.0, 60.0), Pt::new(1080.0, 180.0)] }, Style { fill: Rgba::rgb(0x2E, 0x9E, 0x4F), stroke: Rgba::BLACK, width: 3.0, ..Style::default() });
    let text = Style {
        font_size: 44.0,
        bold: true,
        text_color: Rgba::WHITE,
        outline: Rgba::BLACK,
        outline_width: 3.0,
        underline: true,
        width: 0.0,
        align: Align::Center,
        valign: VAlign::Middle,
        ..Style::default()
    };
    d.add(Kind::Text { rect: Box2::from_xywh(360.0, 220.0, 420.0, 120.0), text: "Outlined, underlined".into() }, text.clone());
    d.add(Kind::Text { rect: Box2::from_xywh(360.0, 360.0, 420.0, 80.0), text: "Struck through".into() }, Style { outline_width: 0.0, underline: false, strike: true, text_color: Rgba::BLACK, font_size: 34.0, ..text });
    let callout = Style { fill: blue, fill2: Rgba::rgb(0x0B, 0x3D, 0x91), stroke: blue, width: 3.0, text_color: Rgba::WHITE, font_size: 26.0, align: Align::Center, valign: VAlign::Middle, ..Style::default() };
    d.add(Kind::Callout { rect: Box2::from_xywh(840.0, 240.0, 300.0, 120.0), tail: Pt::new(860.0, 440.0), text: "Gradient callout".into(), more_tails: vec![Pt::new(1100.0, 440.0)], shape: ShapeKind::Rounded, arrow: false }, callout);
    for (i, (label, shape)) in [("1", ShapeKind::Ellipse), ("B", ShapeKind::Rounded), ("iv", ShapeKind::Rectangle)].iter().enumerate() {
        d.add(Kind::Step { center: Pt::new(80.0 + i as f32 * 80.0, 520.0), radius: 26.0, label: label.to_string(), shape: *shape }, Style { fill: Rgba::RED, width: 0.0, text_color: Rgba::WHITE, ..Style::default() });
    }
    for (i, shape) in [ShapeKind::Rectangle, ShapeKind::Rounded, ShapeKind::Ellipse].iter().enumerate() {
        d.add(Kind::Highlight { rect: Box2::from_xywh(360.0 + i as f32 * 150.0, 480.0, 130.0, 60.0), shape: *shape }, Style { fill: Rgba::YELLOW.with_alpha(180), multiply: true, ..Style::default() });
    }
    d.add(Kind::Magnify { rect: Box2::from_xywh(380.0, 600.0, 120.0, 120.0), zoom: 2.5, ellipse: true, offset: Pt::new(260.0, -20.0) }, Style { stroke: Rgba::rgb(0x30, 0x30, 0x30), width: 4.0, shadow: true, ..Style::default() });
    d.add(Kind::Spotlight { rect: Box2::from_xywh(840.0, 560.0, 300.0, 180.0), ellipse: false, dim: 0.5, blur: 0.0, rounded: true }, Style::default());
    io::export(&d, std::path::Path::new(&out)).unwrap();
    println!("wrote {out}");
}
