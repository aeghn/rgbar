#![no_main]

mod blocks;
mod datahodler;
mod prelude;
mod statusbar;
mod util;
mod widgets;
mod window;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::statusbar::StatusBar;
use gtk::gdk::*;
use gtk::prelude::*;
use gtk::*;
use tracing::{info, warn};

/// Called upon application startup.
#[no_mangle]
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_line_number(true)
        .with_target(true)
        .init();

    info!("Building application...");

    let application = gtk::Application::new(None, gio::ApplicationFlags::default());
    info!("Loading CSS...");
    let _style_path = PathBuf::new();
    application.connect_startup(|_| load_css());

    info!("Creating viewport...");

    application.connect_activate(|app| {
        let mut statusbar = StatusBar::new(app);

        statusbar.handle_monitors();

        let self_arc = Arc::new(Mutex::new(statusbar));
        let screen = gdk::Screen::default().expect("Failed to get the default screen.");

        screen.connect_monitors_changed(move |m| {
            warn!("monitor changed");
            let mut self_lock = self_arc.lock().unwrap();
            (*self_lock).check_monitors(m);
        });
    });

    info!("Start.");
    let _args: Vec<String> = vec![];
    application.run_with_args(&_args);
}

pub fn load_css() {
    let provider = CssProvider::new();
    // 0.2.8: Allow for defining the name of the stylesheet to look up

    provider
        .load_from_data(include_bytes!("../res/style.css"))
        .unwrap();

    // Add the provider to the default screen
    StyleContext::add_provider_for_screen(
        &Screen::default().unwrap(),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
