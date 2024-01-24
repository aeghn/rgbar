use crate::datahodler::channel::DualChannel;
use crate::datahodler::channel::MReceiver;
use crate::datahodler::channel::SSender;

use self::common::get_battery_info;
use self::ideapad::get_conservation_mode;
use self::ideapad::ConvervationMode;

use super::Block;
use crate::utils::gtk_icon_loader;
use crate::utils::gtk_icon_loader::IconName;
use glib::clone;
use glib::Cast;
use glib::MainContext;
use gtk::prelude::ContainerExt;
use gtk::prelude::LabelExt;
use gtk::prelude::StyleContextExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::{BoxExt, ImageExt};
use tracing::warn;

mod common;
mod ideapad;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PowerStatus {
    NotCharging = 1,
    Discharging = 2,
    Charging = 3,
    Unknown = 4,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BatteryInfo {
    pub name: String,
    pub status: PowerStatus,
    pub present: u8,
    pub technology: String,
    pub cycle_count: u32,
    pub voltage_min_design: u32,
    pub voltage_now: u32,
    pub power_now: u32,
    pub energy_full_design: u32,
    pub energy_full: u32,
    pub energy_now: u32,
    pub capacity: u8,
    pub capacity_level: String,
    pub model_name: String,
    pub manufacturer: String,
    pub serial_numer: String,
}

impl BatteryInfo {
    pub fn get_percent(&self) -> u8 {
        // (self.energy_now * 100 / self.energy_full).try_into().unwrap()
        return self.capacity;
    }
}



#[derive(Clone)]
pub enum BatteryOut {
    ConvervationMode(ConvervationMode),
    BatteryInfo(BatteryInfo),
    UnknownBatteryInfo,
}

#[derive(Clone)]
pub enum BatteryIn {}

pub struct BatteryBlock {
    dualchannel: DualChannel<BatteryOut, BatteryIn>,
}

impl BatteryBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(100);

        Self { dualchannel }
    }
}

impl Block for BatteryBlock {
    type Out = BatteryOut;
    type In = BatteryIn;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();
        glib::timeout_add_seconds(
            1,
            clone!(@strong sender => move || {
                match get_battery_info() {
                    Ok(info) => sender.send(Self::Out::BatteryInfo(info)).expect("msg"),
                    Err(_) => sender.send(Self::Out::UnknownBatteryInfo).expect("todo"),
                };

                sender.send(BatteryOut::ConvervationMode(get_conservation_mode())).unwrap();

                glib::ControlFlow::Continue
            }),
        );

        let receiver = self.dualchannel.get_in_recevier();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(_) => {}
                    Err(msg) => {
                        warn!("got error msg: {}", msg)
                    }
                }
            }
        });

        Ok(())
    }

    fn get_channel(&self) -> (&SSender<Self::In>, &MReceiver<Self::Out>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let battery_status_image =
            gtk::Image::from_pixbuf(Some(&gtk_icon_loader::load_pixbuf(IconName::BatteryUnk)));

        let battery_percent_value = gtk::Label::builder().build();
        battery_percent_value
            .style_context()
            .add_class("battery-label");

        let convervation_image = gtk::Image::from_pixbuf(Some(&gtk_icon_loader::load_pixbuf_at(
            IconName::BatteryCMUnk,
            10,
        )));
        let power_status_image = gtk::Image::from_pixbuf(Some(&gtk_icon_loader::load_pixbuf_at(
            IconName::BatteryPSDisconnected,
            10,
        )));

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::Center)
            .build();
        vbox.add(&power_status_image);
        vbox.add(&convervation_image);

        holder.pack_start(&battery_status_image, false, false, 0);
        holder.pack_start(&vbox, false, false, 0);
        holder.pack_start(&battery_percent_value, false, false, 0);

        let mut percent = 0;
        let mut cm_status = ConvervationMode::Unknown;
        let mut power_status = PowerStatus::Unknown;

        let mut receiver = self.dualchannel.get_out_receiver();

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        BatteryOut::ConvervationMode(cm) => {
                            if cm_status != cm {
                                cm_status = cm;
                                let mapped = match cm_status {
                                    ConvervationMode::Enable => IconName::BatteryCMOn,
                                    ConvervationMode::Disable => IconName::BatteryCMOff,
                                    ConvervationMode::Unknown => IconName::BatteryCMUnk,
                                };
                                convervation_image.set_from_pixbuf(Some(
                                    &gtk_icon_loader::load_pixbuf_at(mapped, 10),
                                ));
                            }
                        }
                        BatteryOut::BatteryInfo(bi) => {
                            let status = bi.get_percent();

                            if status != percent {
                                let mapped = match status {
                                    0..=9 => IconName::BatteryEmpty,
                                    10..=30 => IconName::BatteryLow,
                                    31..=60 => IconName::BatteryMid,
                                    61..=99 => IconName::BatteryHigh,
                                    _ => IconName::BatteryFull,
                                };

                                battery_status_image
                                    .set_from_pixbuf(Some(&gtk_icon_loader::load_pixbuf(mapped)));

                                percent = status;
                            }

                            battery_percent_value.set_label(format!("{}%", status).as_str());

                            let pstatus = bi.status;

                            if power_status != pstatus {
                                power_status = pstatus;
                                let mapped = match power_status {
                                    PowerStatus::NotCharging => IconName::BatteryPSNotCharging,
                                    PowerStatus::Discharging => IconName::BatteryPSDisconnected,
                                    PowerStatus::Charging => IconName::BattetyPSCharging,
                                    PowerStatus::Unknown => IconName::BatteryPSUnk,
                                };

                                power_status_image.set_from_pixbuf(Some(
                                    &gtk_icon_loader::load_pixbuf_at(mapped, 10),
                                ));
                            }
                        }
                        BatteryOut::UnknownBatteryInfo => todo!(),
                    }
                }
            }
        });

        holder.upcast()
    }
}
