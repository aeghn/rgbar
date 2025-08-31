#![no_main]

mod application;
mod blocks;
pub mod config;
mod datahodler;
mod prelude;
mod util;
mod widgets;
mod window;
use crate::prelude::*;

use std::path::PathBuf;

use crate::application::RGBApplication;
use config::set_config;
use log::info;
use tracing_subscriber::EnvFilter;

/// Called upon application startup.
#[no_mangle]
fn main() -> EResult {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_target(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    set_config()?;

    info!("Building application...");

    let application = gtk::Application::new(None, ApplicationFlags::default());

    info!("Loading CSS...");
    let _style_path = PathBuf::new();

    application.connect_activate(|app| {
        info!("Activating application.");
        let screen = Screen::default().expect("Failed to get the default screen.");
        load_css(&screen);

        RGBApplication::monitor_monitors(&screen, app).unwrap();
    });
    info!("hold on.");
    let _holder = application.hold();

    info!("Start GTK Application.");
    let _args: Vec<String> = vec![];
    application.run_with_args(&_args);

    Ok(())
}

pub fn load_css(screen: &Screen) {
    let provider = CssProvider::new();
    provider
        .load_from_data(include_bytes!("../res/style.css"))
        .unwrap();

    // Add the provider to the screen
    StyleContext::add_provider_for_screen(screen, &provider, STYLE_PROVIDER_PRIORITY_APPLICATION);
}
