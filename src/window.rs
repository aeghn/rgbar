use gtk::gdk::*;
use gtk::prelude::*;
use gtk::*;
use gtk_layer_shell::Edge;
use gtk_layer_shell::LayerShell;

pub(crate) fn create_window(application: &Application, monitor_num: i32) -> ApplicationWindow {
    let window = ApplicationWindow::new(application);

    window.init_layer_shell();
    window.set_layer(gtk_layer_shell::Layer::Top);
    window.auto_exclusive_zone_enable();

    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, false);
    window.set_namespace("gtk-layer-shell");

    let display = Display::default().unwrap();
    let monitor = display.monitor(monitor_num).unwrap();
    window.set_monitor(&monitor);
    window.set_app_paintable(true);

    window
}
