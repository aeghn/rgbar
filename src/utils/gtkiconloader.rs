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
    BatteryUnk,

    BattetyPSCharging,
    BatteryPSNotCharging,
    BatteryPSDisconnected,
    BatteryPSUnk,

    BatteryCMOn,
    BatteryCMOff,
    BatteryCMUnknown,

    Headphone,
    Headset,

    VolumeHigh,
    VolumeMidium,
    VolumeLow,
    VolumeMute,
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
        IconName::BatteryUnk => "",
        IconName::BattetyPSCharging => "",
        IconName::BatteryPSNotCharging => "",
        IconName::BatteryPSDisconnected => "",
        IconName::BatteryPSUnk => "",
        IconName::BatteryCMOn => "",
        IconName::BatteryCMOff => "",
        IconName::BatteryCMUnknown => "",
        IconName::Headphone => "",
        IconName::Headset => "",
        IconName::VolumeHigh => "",
        IconName::VolumeMidium => "",
        IconName::VolumeLow => "",
        IconName::VolumeMute => "",
    }
}

pub fn load_image_at(icon_name: IconName, _size: i32) -> gtk::Label {
    let label = gtk::Label::builder().label(load_label(icon_name)).build();
    label.style_context().add_class("lucide");
    label
}

pub fn load_pixbuf_at(icon_name: IconName, size: i32) -> Pixbuf {
    let fc = |bytes: &'static [u8]| {
        let mis = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from(bytes));

        let buf = Pixbuf::from_stream_at_scale(
            &gtk::gio::InputStream::from(mis),
            size,
            size,
            true,
            None::<&gtk::gio::Cancellable>,
        )
        .unwrap();

        buf
    };

    match icon_name {
        IconName::CPU => fc(include_bytes!("../../res/icons/cpu.svg")),
        IconName::RAM => fc(include_bytes!("../../res/icons/memory.svg")),
        IconName::WIFI => fc(include_bytes!("../../res/icons/wifi.svg")),

        IconName::BatteryFull => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryHigh => fc(include_bytes!("../../res/icons/battery-high.svg")),
        IconName::BatteryMid => fc(include_bytes!("../../res/icons/battery-medium.svg")),
        IconName::BatteryLow => fc(include_bytes!("../../res/icons/battery-low.svg")),
        IconName::BatteryEmpty => fc(include_bytes!("../../res/icons/battery-empty.svg")),
        IconName::BatteryUnk => fc(include_bytes!("../../res/icons/battery-empty.svg")),

        IconName::BattetyPSCharging => fc(include_bytes!("../../res/icons/battery-charging.svg")),
        IconName::BatteryPSNotCharging => {
            fc(include_bytes!("../../res/icons/battery-connected.svg"))
        }
        IconName::BatteryPSDisconnected => {
            fc(include_bytes!("../../res/icons/battery-disconnected.svg"))
        }
        IconName::BatteryPSUnk => fc(include_bytes!("../../res/icons/battery-connected.svg")),

        IconName::BatteryCMOn => fc(include_bytes!("../../res/icons/battery-conser.svg")),
        IconName::BatteryCMOff => fc(include_bytes!("../../res/icons/battery-not-conser.svg")),
        IconName::BatteryCMUnknown => fc(include_bytes!("../../res/icons/battery-not-conser.svg")),

        IconName::Headphone => fc(include_bytes!(
            "../../res/icons/audio-headphones-symbolic.svg"
        )),
        IconName::Headset => fc(include_bytes!("../../res/icons/audio-headset-symbolic.svg")),
        IconName::VolumeHigh => fc(include_bytes!(
            "../../res/icons/audio-volume-high-symbolic.svg"
        )),
        IconName::VolumeMidium => fc(include_bytes!(
            "../../res/icons/audio-volume-medium-symbolic.svg"
        )),
        IconName::VolumeLow => fc(include_bytes!(
            "../../res/icons/audio-volume-low-symbolic.svg"
        )),
        IconName::VolumeMute => fc(include_bytes!(
            "../../res/icons/audio-volume-muted-symbolic.svg"
        )),
    }
}

pub fn load_pixbuf(icon_name: IconName) -> Pixbuf {
    load_pixbuf_at(icon_name, 20)
}