use gtk::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::renderer::Renderer;

pub struct PdfRenderer {
    scrolled: gtk::ScrolledWindow,
    container: gtk::Box,
    document: Rc<RefCell<Option<poppler::Document>>>,
    areas: Rc<RefCell<Vec<gtk::DrawingArea>>>,
    zoom: Rc<RefCell<f64>>,
}

impl PdfRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &["pdf"]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_halign(gtk::Align::Center);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&container)
            .build();

        Self {
            scrolled,
            container,
            document: Rc::new(RefCell::new(None)),
            areas: Rc::new(RefCell::new(Vec::new())),
            zoom: Rc::new(RefCell::new(1.0)),
        }
    }

    fn clear(&self) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }
        self.areas.borrow_mut().clear();
    }

    fn show_error(&self, message: &str) {
        let label = gtk::Label::new(Some(message));
        label.set_wrap(true);
        label.set_margin_top(24);
        label.set_margin_bottom(24);
        self.container.append(&label);
    }
}

impl Renderer for PdfRenderer {
    fn widget(&self) -> gtk::Widget {
        self.scrolled.clone().upcast()
    }

    fn load(&self, path: &Path) {
        self.clear();

        let uri = match glib::filename_to_uri(path, None) {
            Ok(u) => u,
            Err(e) => {
                self.show_error(&format!("Failed to build file URI: {e}"));
                return;
            }
        };

        let document = match poppler::Document::from_file(&uri, None) {
            Ok(d) => d,
            Err(e) => {
                self.show_error(&format!("Failed to load PDF: {e}"));
                return;
            }
        };

        let n_pages = document.n_pages();
        self.document.replace(Some(document));

        for index in 0..n_pages {
            let area = gtk::DrawingArea::new();
            area.set_halign(gtk::Align::Center);

            // Initial size from the page at current zoom
            if let Some(page) = self
                .document
                .borrow()
                .as_ref()
                .and_then(|d| d.page(index))
            {
                let (w, h) = page.size();
                let zoom = *self.zoom.borrow();
                area.set_content_width((w * zoom) as i32);
                area.set_content_height((h * zoom) as i32);
            }

            let document_ref = self.document.clone();
            let zoom_ref = self.zoom.clone();
            let page_index = index;
            area.set_draw_func(move |_area, ctx, _width, _height| {
                let zoom = *zoom_ref.borrow();
                let doc_borrow = document_ref.borrow();
                let Some(doc) = doc_borrow.as_ref() else {
                    return;
                };
                let Some(page) = doc.page(page_index) else {
                    return;
                };

                // Paint a white page background so transparent PDFs aren't
                // rendered on whatever the window background is.
                let (pw, ph) = page.size();
                ctx.save().ok();
                ctx.scale(zoom, zoom);
                ctx.set_source_rgb(1.0, 1.0, 1.0);
                ctx.rectangle(0.0, 0.0, pw, ph);
                let _ = ctx.fill();
                page.render(ctx);
                ctx.restore().ok();
            });

            self.container.append(&area);
            self.areas.borrow_mut().push(area);
        }

        if n_pages == 0 {
            self.show_error("PDF contains no pages.");
        }
    }

    fn set_zoom(&self, level: f64) {
        let level = level.max(0.1);
        *self.zoom.borrow_mut() = level;

        let doc_borrow = self.document.borrow();
        let Some(doc) = doc_borrow.as_ref() else {
            return;
        };

        for (idx, area) in self.areas.borrow().iter().enumerate() {
            if let Some(page) = doc.page(idx as i32) {
                let (w, h) = page.size();
                area.set_content_width((w * level) as i32);
                area.set_content_height((h * level) as i32);
            }
            area.queue_resize();
            area.queue_draw();
        }
    }
}

