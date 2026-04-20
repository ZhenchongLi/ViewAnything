use gtk::prelude::*;
use std::path::PathBuf;

pub fn attach_drop_target<F>(widget: &gtk::Widget, callback: F)
where
    F: Fn(Vec<PathBuf>) + 'static,
{
    let drop_target = gtk::DropTarget::new(
        gdk::FileList::static_type(),
        gdk::DragAction::COPY,
    );

    drop_target.connect_drop(move |_, value, _, _| {
        if let Ok(file_list) = value.get::<gdk::FileList>() {
            let paths: Vec<PathBuf> = file_list
                .files()
                .iter()
                .filter_map(|f| f.path())
                .collect();
            if !paths.is_empty() {
                callback(paths);
                return true;
            }
        }
        false
    });

    widget.add_controller(drop_target);
}
