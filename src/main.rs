#![no_main]

mod blocks;
mod constants;
mod statusbar;
mod utils;
mod window;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gdk_pixbuf::gio::ApplicationFlags;
use tracing_subscriber::prelude::*;

use crate::statusbar::StatusBar;
use gtk::gdk::*;
use gtk::prelude::*;
use gtk::*;
use tracing::{info, warn};

/// Called upon application startup.
#[no_mangle]
fn main() {
    tracing_subscriber::registry()
        .with(tracing_tree::HierarchicalLayer::new(2))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Building application...");
    let application = gtk::Application::new(None, ApplicationFlags::default());
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
    application.run();
}

pub fn load_css() {
    let provider = CssProvider::new();
    // 0.2.8: Allow for defining the name of the stylesheet to look up

    provider
        .load_from_data(include_bytes!("../config/style.css"))
        .expect("");

    // Add the provider to the default screen
    StyleContext::add_provider_for_screen(
        &Screen::default().expect(""),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
