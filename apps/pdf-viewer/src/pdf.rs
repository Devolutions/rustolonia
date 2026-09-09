use pdf_oxide::document::PdfDocument;
use pdf_oxide::rendering::{render_page_fit, ImageFormat, RenderOptions};
use pdf_oxide::search::{SearchOptions, TextSearcher};
use pdf_oxide::writer::DocumentBuilder;
use pdf_oxide::{Destination, OutlineItem};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const PREVIEW_WIDTH_PX: u32 = 900;
const PREVIEW_HEIGHT_PX: u32 = 1100;
const PREVIEW_MAX_OUTPUT_PIXELS: u64 = 4_000_000;
const MAX_EXTRACTED_TEXT_BYTES: usize = 64 * 1024;

pub type PdfHandle = Arc<Mutex<PdfDocument>>;

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub page: usize,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct OutlineEntry {
    pub title: String,
    pub page: Option<usize>,
    pub children: Vec<OutlineEntry>,
}

impl OutlineEntry {
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(Self::count).sum::<usize>()
    }
}

pub fn outline_len(entries: &[OutlineEntry]) -> usize {
    entries.iter().map(OutlineEntry::count).sum()
}

#[derive(Clone)]
pub struct DocumentState {
    pub pdf: PdfHandle,
    pub path: PathBuf,
    pub name: String,
    pub page: usize,
    pub page_count: usize,
    pub image: Vec<u8>,
    pub text: String,
    pub outline: Vec<OutlineEntry>,
}

pub fn load_page(path: &Path, page: usize) -> std::result::Result<DocumentState, String> {
    let document = Arc::new(Mutex::new(
        PdfDocument::open(path).map_err(|error| error.to_string())?,
    ));
    let outline = read_outline(&document);
    render_page(document, path, page, outline)
}

pub fn read_outline(document: &PdfHandle) -> Vec<OutlineEntry> {
    let Ok(guard) = document.lock() else {
        return Vec::new();
    };
    match guard.get_outline() {
        Ok(Some(items)) => items
            .into_iter()
            .map(|item| convert_outline_item(&guard, item))
            .collect(),
        _ => Vec::new(),
    }
}

fn convert_outline_item(document: &PdfDocument, item: OutlineItem) -> OutlineEntry {
    let title = {
        let trimmed = item.title.trim();
        if trimmed.is_empty() {
            "(Untitled bookmark)".to_string()
        } else {
            trimmed.to_string()
        }
    };
    let page = match item.dest {
        Some(Destination::PageIndex(index)) => Some(index),
        Some(Destination::Named(name)) => document.resolve_named_destination(&name).ok().flatten(),
        None => None,
    };
    OutlineEntry {
        title,
        page,
        children: item
            .children
            .into_iter()
            .map(|child| convert_outline_item(document, child))
            .collect(),
    }
}

fn clamp_outline(entries: Vec<OutlineEntry>, page_count: usize) -> Vec<OutlineEntry> {
    entries
        .into_iter()
        .map(|mut entry| {
            if entry.page.is_some_and(|page| page >= page_count) {
                entry.page = None;
            }
            entry.children = clamp_outline(entry.children, page_count);
            entry
        })
        .collect()
}

pub fn render_page(
    document: PdfHandle,
    path: &Path,
    page: usize,
    outline: Vec<OutlineEntry>,
) -> std::result::Result<DocumentState, String> {
    let (page_count, rendered, text) = {
        let document_guard = document
            .lock()
            .map_err(|_| "PDF document lock poisoned".to_string())?;
        let page_count = document_guard
            .page_count()
            .map_err(|error| error.to_string())?;
        if page >= page_count {
            return Err(format!(
                "page {} is outside the document's {} pages",
                page + 1,
                page_count
            ));
        }

        let mut options = RenderOptions::with_dpi(144);
        options.max_output_pixels = PREVIEW_MAX_OUTPUT_PIXELS;
        let rendered = render_page_fit(
            &document_guard,
            page,
            PREVIEW_WIDTH_PX,
            PREVIEW_HEIGHT_PX,
            &options,
        )
        .map_err(|error| error.to_string())?;
        if rendered.format != ImageFormat::Png {
            return Err("PDF renderer returned a non-PNG page".to_string());
        }
        let text = document_guard
            .extract_text(page)
            .map(limit_text)
            .unwrap_or_else(|_| "No extractable text on this page.".to_string());
        (page_count, rendered, text)
    };
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("document.pdf")
        .to_string();

    Ok(DocumentState {
        pdf: document,
        path: path.to_path_buf(),
        name,
        page,
        page_count,
        image: rendered.data,
        text,
        outline: clamp_outline(outline, page_count),
    })
}

pub fn search_document(
    document: &PdfHandle,
    query: &str,
) -> std::result::Result<Vec<SearchHit>, String> {
    let document_guard = document
        .lock()
        .map_err(|_| "PDF document lock poisoned".to_string())?;
    let options = SearchOptions::new()
        .with_case_insensitive(true)
        .with_literal(true)
        .with_max_results(5_000);
    TextSearcher::search(&document_guard, query, &options)
        .map(|results| {
            results
                .into_iter()
                .map(|result| SearchHit {
                    page: result.page,
                    text: result.text,
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

fn limit_text(mut text: String) -> String {
    if text.len() <= MAX_EXTRACTED_TEXT_BYTES {
        return text;
    }
    let mut end = MAX_EXTRACTED_TEXT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str("\n\n[Text preview truncated for responsiveness.]");
    text
}

pub fn create_sample_pdf(path: &Path) -> std::result::Result<(), String> {
    let mut markdown = String::from(
        "# Rustolonia PDF Viewer\n\n\
         This sample document is generated with PDF Oxide and rendered in the \
         Rustolonia Avalonia consumer.\n\n",
    );
    for page in 1..=4 {
        markdown.push_str(&format!("## Sample section {page}\n\n"));
        for line in 1..=18 {
            markdown.push_str(&format!(
                "Page {page}, line {line}: PDF Oxide extracts this text and rasterizes the same page for the viewer.\n\n"
            ));
        }
    }

    let mut builder = DocumentBuilder::new().bookmark_tree(|outline| {
        use pdf_oxide::writer::OutlineItem;
        outline.add_item(OutlineItem::new("Sample section 1", 0));
        outline.add_item(OutlineItem::new("Sample section 2", 1));
        outline.add_item(OutlineItem::new("Contents", 0));
        outline.add_child(OutlineItem::new("Overview", 0));
        outline.add_child(OutlineItem::new("Details", 1));
        outline.pop();
    });
    builder
        .a4_page()
        .heading(1, "Rustolonia PDF Viewer")
        .paragraph(
            "This sample document is generated with PDF Oxide and rendered in the Rustolonia Avalonia consumer.",
        )
        .paragraph(&markdown)
        .done();
    let pdf = builder.build().map_err(|error| error.to_string())?;
    std::fs::write(path, pdf).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sample_pdf_is_multipage_and_renderable() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rustolonia-pdf-viewer-test-{}-{suffix}.pdf",
            std::process::id()
        ));

        create_sample_pdf(&path).expect("create sample PDF");
        let first_page = load_page(&path, 0).expect("render first page");
        let later_page = render_page(
            first_page.pdf.clone(),
            &path,
            first_page.page_count.saturating_sub(1),
            first_page.outline.clone(),
        )
        .expect("render a later page from the reused document");
        let matches = search_document(&first_page.pdf, "Page 4").expect("search sample PDF");

        assert!(first_page.page_count >= 2);
        assert!(!first_page.image.is_empty());
        assert!(first_page.text.contains("Rustolonia PDF Viewer"));
        assert_eq!(first_page.page_count - 1, later_page.page);
        assert!(!later_page.image.is_empty());
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|result| result.page > 0));
        assert!(
            first_page
                .outline
                .iter()
                .any(|entry| entry.title == "Sample section 2" && entry.page == Some(1)),
            "sample PDF should expose Acrobat-style bookmarks"
        );
        assert!(first_page
            .outline
            .iter()
            .any(|entry| entry.title == "Contents" && !entry.children.is_empty()));
        assert_eq!(later_page.outline.len(), first_page.outline.len());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ms_rdpbcgr_outline_is_navigable_when_present() {
        let path = PathBuf::from(r"C:\Users\mamoreau\Downloads\[MS-RDPBCGR].pdf");
        if !path.is_file() {
            return;
        }

        let document = load_page(&path, 0).expect("open MS-RDPBCGR");
        assert!(
            document.page_count >= 2,
            "MS-RDPBCGR should be multi-page"
        );
        let page_text = document.text.to_ascii_lowercase();
        assert!(
            page_text.contains("rdp")
                || page_text.contains("remote desktop")
                || page_text.contains("ms-rdpbcgr"),
            "unexpected page 1 text: {}",
            document.text.chars().take(500).collect::<String>()
        );
        let intro_hits =
            search_document(&document.pdf, "Introduction").expect("search Introduction");
        assert!(
            !intro_hits.is_empty(),
            "MS-RDPBCGR should contain Introduction"
        );
        assert!(
            !document.outline.is_empty(),
            "MS-RDPBCGR should expose a table of contents"
        );
        assert!(
            document
                .outline
                .iter()
                .any(|entry| entry.title == "1 Introduction" && entry.page == Some(16)),
            "MS-RDPBCGR root bookmark should open Introduction"
        );
        let jump = document
            .outline
            .iter()
            .flat_map(flatten_outline)
            .find(|entry| entry.page.is_some_and(|page| page > 0))
            .expect("MS-RDPBCGR should have a bookmark past page 1");
        let page = jump.page.expect("bookmark page");
        let rendered = render_page(document.pdf.clone(), &path, page, document.outline.clone())
            .expect("render bookmarked page");
        assert_eq!(rendered.page, page);
        assert!(!rendered.image.is_empty());
    }

    fn flatten_outline(entry: &OutlineEntry) -> Vec<&OutlineEntry> {
        let mut items = vec![entry];
        for child in &entry.children {
            items.extend(flatten_outline(child));
        }
        items
    }
}
