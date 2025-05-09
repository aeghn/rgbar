use gtk::gdk::ffi::gdk_screen_get_monitor_plug_name;

/// Copied from https://github.com/elkowar/eww/commit/510b824e7545a7c98e050522a7bc93d884c53432#diff-12f72e340b5f9de88e64ebfc3df46f5d14a2bc5b0b2221eadf43ba087765d762R21
/// Get the name of monitor plug for given monitor number
/// workaround gdk not providing this information on wayland in regular calls
/// gdk_screen_get_monitor_plug_name is deprecated but works fine for that case
pub fn get_monitor_plug_name(display: &gtk::gdk::Display, monitor_num: i32) -> Option<&str> {
    unsafe {
        use gtk::gdk::glib::translate::ToGlibPtr;
        let plug_name_pointer = gdk_screen_get_monitor_plug_name(
            display.default_screen().to_glib_none().0,
            monitor_num,
        );
        use std::ffi::CStr;
        if !plug_name_pointer.is_null() {
            let monitor_name = CStr::from_ptr(plug_name_pointer).to_str().ok();
            log::info!("get_monitor_plug_name: {:?}", monitor_name);
            monitor_name
        } else {
            log::error!("unable get monitor plug name by gdk_screen_get_monitor_plug_name");
            None
        }
    }
}
