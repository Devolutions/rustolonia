//! Writes a synthetic "screenshot" (window chrome, buttons, real text) for
//! tests that must not use the screen: `--example synthetic -- OUT.png`.

use jiffshot_doc::object::{Kind, ShapeKind, Style};
use jiffshot_doc::text::Align;
use jiffshot_doc::{io, Box2, Document, Rgba};

fn main() {
    let out = std::env::args().nth(1).expect("usage: synthetic OUT.png");
    let mut doc = Document::blank(1600, 1000, Rgba::rgb(0xE9, 0xEC, 0xF1));
    let plain = |fill: Rgba| Style { fill, stroke: Rgba::TRANSPARENT, width: 0.0, ..Style::default() };
    let text = |color: Rgba, size: f32, bold: bool| Style {
        fill: Rgba::TRANSPARENT,
        width: 0.0,
        text_color: color,
        font_size: size,
        bold,
        align: Align::Left,
        ..Style::default()
    };
    doc.add(Kind::Shape { rect: Box2::from_xywh(80.0, 60.0, 1440.0, 880.0), shape: ShapeKind::Rectangle }, plain(Rgba::WHITE));
    doc.add(Kind::Shape { rect: Box2::from_xywh(80.0, 60.0, 1440.0, 56.0), shape: ShapeKind::Rectangle }, plain(Rgba::rgb(0x20, 0x30, 0x50)));
    doc.add(Kind::Text { rect: Box2::from_xywh(100.0, 66.0, 600.0, 44.0), text: "Project Settings".into() }, text(Rgba::WHITE, 24.0, true));
    doc.add(Kind::Shape { rect: Box2::from_xywh(80.0, 116.0, 300.0, 824.0), shape: ShapeKind::Rectangle }, plain(Rgba::rgb(0xF3, 0xF4, 0xF7)));
    for (i, item) in ["General", "Accounts", "Notifications", "Privacy", "Storage", "About"].iter().enumerate() {
        doc.add(Kind::Text { rect: Box2::from_xywh(110.0, 150.0 + i as f32 * 56.0, 260.0, 40.0), text: item.to_string() }, text(Rgba::rgb(0x33, 0x38, 0x44), 22.0, i == 0));
    }
    let body = [
        "Choose where captures are saved and how they are named.",
        "Jiffshot keeps every capture in the library folder.",
        "Edits are saved next to the original image.",
    ];
    for (i, line) in body.iter().enumerate() {
        doc.add(Kind::Text { rect: Box2::from_xywh(430.0, 160.0 + i as f32 * 48.0, 1000.0, 40.0), text: line.to_string() }, text(Rgba::rgb(0x22, 0x26, 0x2E), 22.0, false));
    }
    for (i, (label, color)) in [("Save", Rgba::rgb(0x1E, 0x6F, 0xD9)), ("Cancel", Rgba::rgb(0x6B, 0x72, 0x80)), ("Delete", Rgba::rgb(0xD9, 0x30, 0x25))].iter().enumerate() {
        let x = 430.0 + i as f32 * 190.0;
        doc.add(Kind::Shape { rect: Box2::from_xywh(x, 420.0, 170.0, 56.0), shape: ShapeKind::Rounded }, plain(*color));
        doc.add(Kind::Text { rect: Box2::from_xywh(x, 428.0, 170.0, 44.0), text: label.to_string() }, Style { align: Align::Center, ..text(Rgba::WHITE, 22.0, true) });
    }
    for (i, c) in [Rgba::rgb(0xF5, 0xB4, 0x00), Rgba::rgb(0x2E, 0x9E, 0x4F), Rgba::rgb(0x7B, 0x4C, 0xC9)].iter().enumerate() {
        let x = 430.0 + i as f32 * 260.0;
        doc.add(Kind::Shape { rect: Box2::from_xywh(x, 560.0, 220.0, 300.0), shape: ShapeKind::Rounded }, plain(*c));
        doc.add(Kind::Shape { rect: Box2::from_xywh(x + 60.0, 620.0, 100.0, 100.0), shape: ShapeKind::Ellipse }, plain(Rgba::WHITE));
    }
    doc.add(
        Kind::Text { rect: Box2::from_xywh(430.0, 880.0, 1000.0, 40.0), text: "Support: jane.doe@example.com or +1 514 555 0199".into() },
        text(Rgba::rgb(0x22, 0x26, 0x2E), 22.0, false),
    );
    doc.flatten_objects();
    io::export(&doc, std::path::Path::new(&out)).unwrap();
    println!("wrote {out}");
}
