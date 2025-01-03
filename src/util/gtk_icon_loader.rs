use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gdk::prelude::InputStreamExt;
use gtk::prelude::IconThemeExt;

#[derive(Clone)]
pub struct GtkIconLoader {
    cache: Rc<RefCell<HashMap<String, gtk::gdk_pixbuf::Pixbuf>>>,
}

impl std::fmt::Debug for GtkIconLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GtkIconLoader").finish()
    }
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
    BatteryPowerFull,

    BatteryConservationOn,
    BatteryConservationOff,
    BatteryConservationUnknown,

    Headphone,

    VolumeHigh,
    VolumeMedium,
    VolumeLow,
    VolumeMute,
}

impl GtkIconLoader {
    pub fn new() -> Self {
        GtkIconLoader {
            cache: RefCell::new(HashMap::new()).into(),
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

    pub fn load_from_name(&self, key: &str) -> Option<gdk::gdk_pixbuf::Pixbuf> {
        let key = Self::map_name(key);
        match self.cache.borrow().get(key) {
            None => {}
            Some(image) => {
                return Some(image.clone());
            }
        }

        let icon_theme = gtk::IconTheme::default().unwrap();
        let icon = icon_theme.load_icon(key, 24, gtk::IconLookupFlags::FORCE_SVG);
        if let Ok(Some(pbf)) = icon
            .map(|pbf| pbf.and_then(|p| p.scale_simple(24, 24, gdk::gdk_pixbuf::InterpType::Hyper)))
        {
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

fn read_into_pixbuf(svg_data: &str, width: i32, height: i32) -> gtk::gdk_pixbuf::Pixbuf {
    let stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from(svg_data.as_bytes()));
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale(&stream, width, height, true, None::<&gtk::gio::Cancellable>).unwrap();
    stream.close(None::<&gtk::gio::Cancellable>).unwrap();

    pixbuf
}

#[macro_export]
macro_rules! include_surface {
    ($path:expr, $width:expr, $height:expr) => {{
        let data = include_str!($path);
        read_into_pixbuf(data, $width, $height)
    }};
}

pub fn load_label(icon_name: IconName) -> gdk::gdk_pixbuf::Pixbuf {
    const BASE_SIZE: i32 = 18;
    let pixbuf = match icon_name {
        IconName::CPU => include_surface!(
            "../../res/icons/cpu.svg",
            BASE_SIZE * 10 / 7,
            BASE_SIZE * 8 / 7
        ),
        IconName::RAM => include_surface!(
            "../../res/icons/memory.svg",
            BASE_SIZE * 6 / 5,
            BASE_SIZE * 6 / 5
        ),
        IconName::WIFI => include_surface!("../../res/icons/wifi.svg", BASE_SIZE, BASE_SIZE),
        IconName::BatteryFull => {
            include_surface!(
                "../../res/icons/battery-full.svg",
                BASE_SIZE * 3 / 2,
                BASE_SIZE
            )
        }
        IconName::BatteryHigh => {
            include_surface!(
                "../../res/icons/battery-high.svg",
                BASE_SIZE * 3 / 2,
                BASE_SIZE
            )
        }
        IconName::BatteryMid => {
            include_surface!(
                "../../res/icons/battery-medium.svg",
                BASE_SIZE * 3 / 2,
                BASE_SIZE
            )
        }
        IconName::BatteryLow => include_surface!(
            "../../res/icons/battery-low.svg",
            BASE_SIZE * 3 / 2,
            BASE_SIZE
        ),
        IconName::BatteryEmpty => {
            include_surface!(
                "../../res/icons/battery-empty.svg",
                BASE_SIZE * 3 / 2,
                BASE_SIZE
            )
        }
        IconName::BattetyPowerCharging => {
            include_surface!(
                "../../res/icons/battery-charging.svg",
                BASE_SIZE * 3 / 2,
                BASE_SIZE
            )
        }
        IconName::BatteryPowerNotCharging => {
            include_surface!(
                "../../res/icons/battery-connected.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::BatteryPowerDisconnected => {
            include_surface!(
                "../../res/icons/battery-disconnected.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::BatteryPowerFull => {
            include_surface!("../../res/icons/cpu.svg", BASE_SIZE, BASE_SIZE)
        }
        IconName::BatteryPowerUnknown => {
            include_surface!("../../res/icons/cpu.svg", BASE_SIZE, BASE_SIZE)
        }
        IconName::BatteryConservationOn => {
            include_surface!("../../res/icons/battery-conser.svg", BASE_SIZE, BASE_SIZE)
        }
        IconName::BatteryConservationOff => {
            include_surface!(
                "../../res/icons/battery-not-conser.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::BatteryConservationUnknown => {
            include_surface!(
                "../../res/icons/battery-not-conser.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::Headphone => {
            include_surface!("../../res/icons/audio-headset.svg", BASE_SIZE, BASE_SIZE)
        }
        IconName::VolumeHigh => {
            include_surface!(
                "../../res/icons/audio-volume-high.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::VolumeMedium => {
            include_surface!(
                "../../res/icons/audio-volume-medium.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
        IconName::VolumeLow => {
            include_surface!("../../res/icons/audio-volume-low.svg", BASE_SIZE, BASE_SIZE)
        }
        IconName::VolumeMute => {
            include_surface!(
                "../../res/icons/audio-volume-muted.svg",
                BASE_SIZE,
                BASE_SIZE
            )
        }
    };
    pixbuf
}

pub fn load_font_icon(icon_name: IconName) -> gtk::Image {
    gtk::Image::from_pixbuf(Some(&load_label(icon_name)))
}
