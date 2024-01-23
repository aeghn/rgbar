use gtk::{
    prelude::{StyleContextExt, WidgetExt},
    Widget,
};

pub mod chart;

pub trait WidgetShortCut {
    fn add_css(&self, css_name: &str);
}

impl WidgetShortCut for Widget {
    fn add_css(&self, css_name: &str) {
        self.style_context().add_class(css_name);
    }
}
