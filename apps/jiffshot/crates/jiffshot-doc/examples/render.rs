//! Renders any file the editor can open to a PNG:
//! `cargo run -p jiffshot-doc --example render -- IN OUT.png`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: render IN OUT.png");
        std::process::exit(2);
    };
    let doc = jiffshot_doc::io::open(std::path::Path::new(input)).unwrap();
    eprintln!("{}x{}, {} objects", doc.width(), doc.height(), doc.objects.len());
    jiffshot_doc::io::export(&doc, std::path::Path::new(output)).unwrap();
}
