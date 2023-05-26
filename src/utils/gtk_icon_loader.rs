use std::collections::HashMap;

use gdk_pixbuf::Pixbuf;
use gtk::{IconTheme, traits::IconThemeExt, Image};

pub struct GtkIconLoader {
    cache: HashMap<String, gtk::Image>,
}

impl GtkIconLoader {
    pub fn new() -> Self {
        GtkIconLoader { cache: HashMap::new() }
    }

    fn map_name(key: &str) -> &str {
        if "code-url-handler".eq_ignore_ascii_case(key) {
            return "code";
        }
        return key;
    }
 
    pub fn load_from_name(&mut self, key: &str) -> Option<&Image> {
        let key = Self::map_name(key);
        if self.cache.contains_key(key) {
            let image = self.cache.get(key).unwrap();
            return Some(image);
        }

        let icon_theme = gtk::IconTheme::default().unwrap();
        let icon: Result<Option<Pixbuf>, glib::Error> = icon_theme.load_icon(key, 22, gtk::IconLookupFlags::FORCE_SVG);
        if let Ok(_p) = icon {
            if let Some(_i) = _p {
                let image = Image::from_pixbuf(Some(&_i));
                self.cache.insert(key.to_string(), image.to_owned());
                let image = self.cache.get(key).unwrap();
                return Some(image);
            } else {
                None
            }
        } else {
            None
        }
    }
}
