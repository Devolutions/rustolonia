//! A contact sheet of effects applied to an image, for checking by eye:
//! `cargo run -p jiffshot-doc --example effects -- IN.png OUT.png`.

use jiffshot_doc::effects::{self, Effect};
use jiffshot_doc::object::{ImageRef, Kind, Style};
use jiffshot_doc::{io, raster, Box2, Document, Rgba};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let src = io::load_image(std::path::Path::new(&args[0])).unwrap();
    let small = raster::resample(&src, 400, (400.0 * src.height() as f32 / src.width() as f32) as u32).unwrap();
    let names = ["Sawtooth edge", "Wave edge", "Torn bottom", "Perspective", "Tilt", "Page curl", "Reflection", "Hue shift", "Capture info"];
    let mut sheet = Document::blank(3 * 440, 3 * 380, Rgba::rgb(0x60, 0x64, 0x6C));
    for (i, name) in names.iter().enumerate() {
        let mut d = Document::new(small.clone());
        let e = match *name {
            "Capture info" => Effect::CaptureInfo { text: "2026-10-28 10:30:00  ·  Project Settings  ·  Windows".into() },
            n => effects::by_name(n, Rgba::RED).unwrap(),
        };
        effects::apply(&mut d, e);
        let flat = d.flatten();
        let (x, y) = ((i % 3) as f32 * 440.0 + 20.0, (i / 3) as f32 * 380.0 + 20.0);
        let rect = Box2::from_xywh(x, y, flat.width() as f32, flat.height() as f32);
        sheet.add(Kind::Image { rect, image: ImageRef::new(name.to_string(), flat) }, Style { width: 0.0, ..Style::default() });
    }
    io::export(&sheet, std::path::Path::new(&args[1])).unwrap();
}
