// compile-flags: --edition 2015
#![allow(unused)]
#![feature(doc_cfg)]

fn main() {
#[cfg(any(windows, doc))]
#[doc(cfg(windows))]
/// The application's icon in the notification area (a.k.a. system tray).
///
/// # Examples
///
/// ```no_run
/// extern crate my_awesome_ui_library;
/// use my_awesome_ui_library::current_app;
/// use my_awesome_ui_library::windows::notification;
///
/// let icon = current_app().get::<notification::Icon>();
/// icon.show();
/// icon.show_message("Hello");
/// ```
pub struct Icon {
    // ...
}
}