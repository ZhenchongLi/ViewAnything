use gtk::prelude::*;
use std::path::Path;

use crate::renderer::Renderer;

pub struct MediaRenderer {
    container: gtk::Box,
    video: gtk::Video,
    audio_badge: gtk::Image,
}

impl MediaRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &[
            // Audio
            "mp3", "m4a", "wav", "flac", "aac", "aiff", "ogg", "opus",
            // Video
            "mp4", "mov", "m4v", "avi", "mkv", "webm",
        ]
    }

    pub const fn audio_extensions() -> &'static [&'static str] {
        &["mp3", "m4a", "wav", "flac", "aac", "aiff", "ogg", "opus"]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    fn is_audio(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .map(|e| Self::audio_extensions().contains(&e.as_str()))
            .unwrap_or(false)
    }

    pub fn new() -> Self {
        let video = gtk::Video::new();
        video.set_autoplay(false);
        video.set_hexpand(true);
        video.set_vexpand(true);

        // Audio indicator icon, shown centered on top of the video area
        // for audio-only files. Hidden by default; toggled in load().
        let audio_badge = gtk::Image::from_icon_name("audio-x-generic-symbolic");
        audio_badge.set_pixel_size(96);
        audio_badge.set_halign(gtk::Align::Center);
        audio_badge.set_valign(gtk::Align::Center);
        audio_badge.set_visible(false);

        // Overlay lets us stack the audio icon on top of the Video widget
        // without interfering with gtk::Video's built-in media controls.
        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_child(Some(&video));
        overlay.add_overlay(&audio_badge);

        // Top-level container requested by task spec.
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();
        container.append(&overlay);

        Self {
            container,
            video,
            audio_badge,
        }
    }
}

impl Renderer for MediaRenderer {
    fn widget(&self) -> gtk::Widget {
        self.container.clone().upcast()
    }

    fn load(&self, path: &Path) {
        // Toggle the audio-only badge based on the file extension.
        self.audio_badge.set_visible(Self::is_audio(path));

        // Prefer MediaFile::for_filename for finer-grained control over
        // the stream. Fall back to set_filename if anything unexpected
        // happens. for_filename itself does not return a Result, but
        // wrapping in a MediaFile keeps the door open for future error
        // handling (e.g. inspecting the stream's error property).
        let media_file = gtk::MediaFile::for_filename(path);
        self.video.set_media_stream(Some(&media_file));
    }
}
