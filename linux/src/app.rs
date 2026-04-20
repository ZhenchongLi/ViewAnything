use adw::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::renderer::RendererFactory;
use crate::window::ViewerWindow;

thread_local! {
    static OPEN_WINDOWS: RefCell<Vec<Rc<ViewerWindow>>> = RefCell::new(Vec::new());
}

pub fn on_activate(app: &adw::Application) {
    show_open_dialog(app);
}

pub fn on_open(app: &adw::Application, files: &[gio::File], _hint: &str) {
    for file in files {
        if let Some(path) = file.path() {
            open_document(app, &path);
        }
    }
}

pub fn open_document(app: &adw::Application, path: &Path) {
    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => path.to_path_buf(),
    };

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    if !RendererFactory::is_supported(&ext) {
        show_error(app, &format!("Unsupported file type: .{}", ext));
        return;
    }

    let already_open = OPEN_WINDOWS.with(|w| {
        w.borrow()
            .iter()
            .find(|win| win.file_path() == path)
            .cloned()
    });
    if let Some(existing) = already_open {
        existing.present();
        return;
    }

    let app_weak = app.downgrade();
    let window = ViewerWindow::new(app, &path);

    let app_for_drop = app.downgrade();
    window.set_on_open_files(move |paths: Vec<PathBuf>| {
        if let Some(app) = app_for_drop.upgrade() {
            for p in paths {
                open_document(&app, &p);
            }
        }
    });

    let path_for_close = path.clone();
    window.set_on_close(move || {
        OPEN_WINDOWS.with(|w| {
            w.borrow_mut().retain(|win| win.file_path() != path_for_close);
        });
        if let Some(app) = app_weak.upgrade() {
            glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                let has_windows = OPEN_WINDOWS.with(|w| !w.borrow().is_empty());
                if !has_windows {
                    app.quit();
                }
            });
        }
    });

    OPEN_WINDOWS.with(|w| w.borrow_mut().push(window.clone()));
    window.present();
}

fn show_open_dialog(app: &adw::Application) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Supported documents"));
    for ext in RendererFactory::all_supported_extensions() {
        filter.add_suffix(ext);
    }

    let dialog = gtk::FileDialog::builder()
        .title("Open Document")
        .modal(false)
        .default_filter(&filter)
        .build();

    // Keep the app alive while the async file dialog is visible —
    // otherwise the main loop quits before the user can pick anything.
    // The returned guard holds the app; dropping it at the end of the
    // callback releases.
    let hold_guard = app.hold();
    let app_clone = app.clone();
    dialog.open(None::<&gtk::Window>, None::<&gio::Cancellable>, move |res| {
        let _release_on_drop = hold_guard;
        match res {
            Ok(file) => {
                if let Some(path) = file.path() {
                    open_document(&app_clone, &path);
                }
            }
            Err(_) => {
                // User cancelled — quit if no windows open
                let has_windows = OPEN_WINDOWS.with(|w| !w.borrow().is_empty());
                if !has_windows {
                    app_clone.quit();
                }
            }
        }
    });
}

fn show_error(app: &adw::Application, message: &str) {
    let dialog = adw::MessageDialog::builder()
        .heading("Error")
        .body(message)
        .modal(true)
        .build();
    dialog.add_response("ok", "OK");
    dialog.present();
    let _ = app;
}
