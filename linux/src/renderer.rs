use gtk::prelude::*;
use std::path::Path;

use crate::image_renderer::ImageRenderer;
use crate::media_renderer::MediaRenderer;
use crate::pdf_renderer::PdfRenderer;
use crate::web_renderer::WebRenderer;

pub trait Renderer {
    fn widget(&self) -> gtk::Widget;
    fn load(&self, path: &Path);
    fn set_zoom(&self, _level: f64) {}
}

pub struct RendererFactory;

impl RendererFactory {
    pub fn renderer_for(ext: &str) -> Box<dyn Renderer> {
        let ext = ext.to_ascii_lowercase();
        if PdfRenderer::supports(&ext) {
            return Box::new(PdfRenderer::new());
        }
        if ImageRenderer::supports(&ext) {
            return Box::new(ImageRenderer::new());
        }
        if MediaRenderer::supports(&ext) {
            return Box::new(MediaRenderer::new());
        }
        Box::new(WebRenderer::new())
    }

    pub fn all_supported_extensions() -> Vec<&'static str> {
        let mut exts: Vec<&'static str> = Vec::new();
        exts.extend(PdfRenderer::extensions());
        exts.extend(ImageRenderer::extensions());
        exts.extend(MediaRenderer::extensions());
        exts.extend(WebRenderer::extensions());
        exts
    }

    pub fn is_supported(ext: &str) -> bool {
        let ext = ext.to_ascii_lowercase();
        Self::all_supported_extensions()
            .iter()
            .any(|e| *e == ext)
    }
}
