use std::collections::HashMap;

use gdk_pixbuf::Pixbuf;

use glib::Bytes;

use gtk::traits::IconThemeExt;
use gtk::Image;

#[derive(Clone)]
pub struct GtkIconLoader {
    cache: HashMap<String, gtk::Image>,
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
    BatteryCMUnk,
}

impl GtkIconLoader {
    pub fn new() -> Self {
        GtkIconLoader {
            cache: HashMap::new(),
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

    pub fn load_from_name(&mut self, key: &str) -> Option<&Image> {
        let key = Self::map_name(key);
        if self.cache.contains_key(key) {
            let image = self.cache.get(key).unwrap();
            return Some(image);
        }

        let icon_theme = gtk::IconTheme::default().unwrap();
        let icon: Result<Option<Pixbuf>, glib::Error> =
            icon_theme.load_icon(key, 22, gtk::IconLookupFlags::FORCE_SVG);
        if let Ok(Some(_i)) = icon {
            let image = Image::from_pixbuf(Some(&_i));
            self.cache.insert(key.to_string(), image.to_owned());
            let image = self.cache.get(key).unwrap();
            return Some(image);
        } else {
            None
        }
    }
}

pub fn load_image(icon_name: IconName) -> Image {
    Image::from_pixbuf(Some(&load_pixbuf(icon_name)))
}

pub fn load_image_at(icon_name: IconName, size: i32) -> Image {
    Image::from_pixbuf(Some(&load_pixbuf_at(icon_name, size)))
}

pub fn load_pixbuf_at(icon_name: IconName, size: i32) -> Pixbuf {
    let fc = |bytes: &'static [u8]| {
        let mis = gio::MemoryInputStream::from_bytes(&Bytes::from(bytes));

        let buf = Pixbuf::from_stream_at_scale(
            &gio::InputStream::from(mis),
            size,
            size,
            true,
            None::<&gio::Cancellable>,
        )
        .unwrap();

        buf
    };

    match icon_name {
        IconName::CPU => fc(include_bytes!("../../res/icons/cpu.svg")),
        IconName::RAM => fc(include_bytes!("../../res/icons/memory.svg")),
        IconName::WIFI => fc(include_bytes!("../../res/icons/wifi.svg")),

        IconName::BatteryFull => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryHigh => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryMid => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryLow => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryEmpty => fc(include_bytes!("../../res/icons/battery-full.svg")),
        IconName::BatteryUnk => fc(include_bytes!("../../res/icons/battery-full.svg")),

        IconName::BattetyPSCharging => fc(include_bytes!("../../res/icons/battery-connected.svg")),
        IconName::BatteryPSNotCharging => {
            fc(include_bytes!("../../res/icons/battery-connected.svg"))
        }
        IconName::BatteryPSDisconnected => {
            fc(include_bytes!("../../res/icons/battery-connected.svg"))
        }
        IconName::BatteryPSUnk => fc(include_bytes!("../../res/icons/battery-connected.svg")),

        IconName::BatteryCMOn => fc(include_bytes!("../../res/icons/battery-conser.svg")),
        IconName::BatteryCMOff => fc(include_bytes!("../../res/icons/battery-conser.svg")),
        IconName::BatteryCMUnk => fc(include_bytes!("../../res/icons/battery-conser.svg")),
    }
}

pub fn load_pixbuf(icon_name: IconName) -> Pixbuf {
    load_pixbuf_at(icon_name, 16)
}
