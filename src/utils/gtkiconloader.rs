use std::cell::RefCell;
use std::collections::HashMap;

use gdk::gdk_pixbuf::Pixbuf;

use gtk::prelude::{StyleContextExt, WidgetExt};
use gtk::traits::IconThemeExt;

#[derive(Clone)]
pub struct GtkIconLoader {
    cache: RefCell<HashMap<String, gtk::gdk_pixbuf::Pixbuf>>,
}

#[derive(PartialEq, Clone)]
pub enum IconName {
    CPU,
    RAM,
    WIFI,

    BatteryFull,
    BatteryHigh,
    BatteryMid,
    BatteryLow,
    BatteryEmpty,

    BattetyPowerCharging,
    BatteryPowerNotCharging,
    BatteryPowerDisconnected,
    BatteryPowerUnknown,

    BatteryConservationOn,
    BatteryConservationOff,
    BatteryConservationUnknown,

    Headphone,

    VolumeHigh,
    VolumeMidium,
    VolumeLow,
    VolumeMute,
    Empty,
}

impl GtkIconLoader {
    pub fn new() -> Self {
        GtkIconLoader {
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn map_name(key: &str) -> &str {
        if "code-url-handler".eq_ignore_ascii_case(key) {
            "code"
        } else if "jetbrains-studio".eq_ignore_ascii_case(key) {
            "androidstudio"
        } else {
            key
        }
    }

    pub fn load_from_name(&self, key: &str) -> Option<Pixbuf> {
        let key = Self::map_name(key);
        match self.cache.borrow().get(key) {
            None => {}
            Some(image) => {
                return Some(image.clone());
            }
        }

        let icon_theme = gtk::IconTheme::default().unwrap();
        let icon = icon_theme.load_icon(key, 18, gtk::IconLookupFlags::FORCE_SVG);
        if let Ok(Some(pbf)) = icon {
            self.cache.borrow_mut().insert(key.to_string(), pbf.clone());
            match self.cache.borrow().get(key) {
                None => None,
                Some(pbf) => {
                    return Some(pbf.clone());
                }
            }
        } else {
            None
        }
    }
}

pub fn load_label(icon_name: IconName) -> &'static str {
    match icon_name {
        IconName::CPU => "",
        IconName::RAM => "",
        IconName::WIFI => "",
        IconName::BatteryFull => "",
        IconName::BatteryHigh => "",
        IconName::BatteryMid => "",
        IconName::BatteryLow => "",
        IconName::BatteryEmpty => "",
        IconName::BattetyPowerCharging => "",
        IconName::BatteryPowerNotCharging => "",
        IconName::BatteryPowerDisconnected => "",
        IconName::BatteryPowerUnknown => "",
        IconName::BatteryConservationOn => "",
        IconName::BatteryConservationOff => "",
        IconName::BatteryConservationUnknown => "",
        IconName::Headphone => "",
        IconName::VolumeHigh => "",
        IconName::VolumeMidium => "",
        IconName::VolumeLow => "",
        IconName::VolumeMute => "",
        IconName::Empty => "",
    }
}

pub fn load_font_icon(icon_name: IconName) -> gtk::Label {
    let label = gtk::Label::builder().label(load_label(icon_name)).build();
    label.style_context().add_class("lucide");
    label
}
