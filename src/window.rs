use gtk::gdk::*;
use gtk::prelude::*;
use gtk::*;
use gtk_layer_shell::Edge;

use crate::constants::ERR_CUSTOM_DRAW;

/// Applies custom visuals.
fn set_visual(window: &ApplicationWindow, screen: Option<&Screen>) {
    if let Some(screen) = screen {
        if let Some(ref visual) = screen.rgba_visual() {
            window.set_visual(Some(visual)); // Needed for transparency, not available in GTK 4+ so
                                             // F.
        }
    }
}

/// Draws the window using a custom color and opacity.
fn draw(_: &ApplicationWindow, ctx: &cairo::Context) -> Inhibit {
    // Fetch config for the values.
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;
    let a = 1.0;

    // Apply
    ctx.set_source_rgba(r, g, b, a);
    ctx.set_operator(cairo::Operator::Screen);
    ctx.paint().expect(ERR_CUSTOM_DRAW);
    Inhibit(false)
}

/// Initializes the status bar.
pub(crate) fn create_window(application: &Application, monitor_num: i32) -> ApplicationWindow {
    // Create a normal GTK window however you like
    let window = ApplicationWindow::new(application);
    window.connect_screen_changed(set_visual);
    window.connect_draw(draw);

    // Initialize layer shell before the window has been fully initialized.
    gtk_layer_shell::init_for_window(&window);

    // Order above normal windows
    // Prior to 0.2.9, this was set to Bottom but 8it caused issues with tooltips being shown below
    // windows.
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Top);

    // Push other windows out of the way
    // Toggling this off may help some if they are in applications that have weird unicode text, which may mess with the bars scaling.
    gtk_layer_shell::auto_exclusive_zone_enable(&window);

    gtk_layer_shell::set_anchor(&window, Edge::Top, true);
    gtk_layer_shell::set_anchor(&window, Edge::Left, true);
    gtk_layer_shell::set_anchor(&window, Edge::Right, true);
    gtk_layer_shell::set_anchor(&window, Edge::Bottom, false);

    // Allows for specifing the namespace of the layer.
    // The default is "gtk-layer-shell" to not break existing configs._
    gtk_layer_shell::set_namespace(&window, "gtk-layer-shell");

    // Initialize gdk::Display by default value, which is decided by the compositor.
    let display = Display::default().expect(crate::constants::ERR_GET_DISPLAY);

    // Gets the actual gdk::Monitor from configured number.
    let monitor = display
        .monitor(monitor_num)
        .expect(crate::constants::ERR_GET_MONITOR);

    // Sets which monitor should be used for the bar.
    gtk_layer_shell::set_monitor(&window, &monitor);

    // For transparency to work.
    window.set_app_paintable(true);

    window
}
