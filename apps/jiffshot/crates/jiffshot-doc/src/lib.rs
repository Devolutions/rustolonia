//! Jiffshot documents: annotation objects over a captured image, their
//! rendering, interactive editing with undo, effects, smart tools, and file
//! formats. There is no UI or platform code here, so all of it is tested
//! headless.

pub mod color;
pub mod combine;
pub mod document;
pub mod editor;
pub mod effects;
pub mod geom;
pub mod io;
pub mod object;
pub mod raster;
pub mod redact;
pub mod smart;
pub mod stamps;
pub mod text;

pub use color::Rgba;
pub use document::Document;
pub use editor::{Editor, Tool};
pub use geom::{Box2, IRect, Pt};
pub use tiny_skia::Pixmap;
