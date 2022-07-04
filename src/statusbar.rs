use gtk::ApplicationWindow;
use gtk::Box;
use gtk::Orientation;
use gtk::traits::BoxExt;
use gtk::traits::ContainerExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;


use crate::blocks;
use crate::blocks::Module;


pub fn build_widgets(window: &ApplicationWindow) {
    let bar = Box::new(Orientation::Horizontal, 10);
    bar.style_context().add_class("bar");

    window.add(&bar);

    let time_but = blocks::time::TimeModule{};
    let hypr_box =  blocks::hyprstatus::HyprStatus{};
    let bat_box = blocks::battery::BatteryModule{};
    let net_box = blocks::netspeed::NetspeedModule{};

    bar.pack_start(&hypr_box.into_widget(),  false, false, 0);
    bar.pack_end(&time_but.into_widget(), false, false, 0);
    bar.pack_end(&bat_box.into_widget(), false, false, 0);
    bar.pack_end(&net_box.into_widget(), false, false, 0);

    window.show_all();
}
