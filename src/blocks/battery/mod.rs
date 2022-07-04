mod common;
mod ideapad;

#[derive(Debug, PartialEq, Eq)]
pub enum PowerStatus {
    NotCharging = 1,
    Discharging = 2,
    Charging = 3,
    Unknown = 4
}

#[derive(Debug)]
pub struct BatteryInfo {
    name : String,
    status : PowerStatus,
    present : u8,
    technology : String,
    cycle_count: u32,
    voltage_min_design: u32,
    voltage_now: u32,
    power_now: u32,
    energy_full_design: u32,
    energy_full: u32,
    energy_now: u32,
    capacity: u8,
    capacity_level: String,
    model_name: String,
    manufacturer : String,
    serial_numer: String,
}


use gtk::{Button, traits::{ButtonExt, WidgetExt, StyleContextExt}};
use tokio::spawn;
use super::Module;

pub struct BatteryModule {

}

fn get_battery() -> String {
    let conservesion_mode;
    if ideapad::is_conservation_mode() {
        conservesion_mode = "";
    } else {
        conservesion_mode = "";
    }
    
    
    let info = common::read_battery_info();
    let status = info.get_status();
    let icon : &str;
    if *status == PowerStatus::Charging {
        icon = "";
    } else if *status == PowerStatus::Discharging {
        icon = "";
    } else if *status == PowerStatus::Unknown {
        icon = "";
    } else if *status == PowerStatus::NotCharging {
        icon = "";
    } else {
        icon = ""
    }

    format!("{}{} {}%", icon, conservesion_mode, info.get_capacity())
}

impl Module<Button> for BatteryModule {
    fn into_widget(self) -> Button {
        let date = gtk::Button::with_label(&get_battery());
        date.style_context().add_class("block");

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            loop {
                let _ = tx.send(get_battery());
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        {
            let date = date.clone();
            rx.attach(None, move |s| {
                date.set_label(s.as_str());
                glib::Continue(true)
            });
        }

        date
    }
}
