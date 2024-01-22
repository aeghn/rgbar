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
        (self.energy_now * 100 / self.energy_full).try_into().unwrap()
    }
}

use crate::datahodler::channel::DualChannel;
use crate::datahodler::channel::MReceiver;
use crate::datahodler::channel::SSender;

use self::common::get_battery_info;
use self::ideapad::get_conservation_mode;
use self::ideapad::ConvervationMode;

use super::Block;
use glib::Cast;
use glib::clone;
use glib::MainContext;
use gtk::false_;
use gtk::prelude::BoxExt;
use gtk::prelude::LabelExt;
use tracing::warn;

#[derive(Clone)]
pub enum BatteryWM {
    ConvervationMode(ConvervationMode),
    BatteryInfo(BatteryInfo),
    UnknownBatteryInfo,
}

#[derive(Clone)]
pub enum BatteryBM {}

pub struct BatteryModule {
    dualchannel: DualChannel<BatteryWM, BatteryBM>,
}

impl BatteryModule {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(100);

        Self { dualchannel }
    }
}

impl Block for BatteryModule {
    type WM = BatteryWM;
    type BM = BatteryBM;

    fn loop_receive(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();
        glib::timeout_add_seconds(
            1,
            clone!(@strong sender => move || {
                match get_battery_info() {
                    Ok(info) => sender.send(Self::WM::BatteryInfo(info)).expect("msg"),
                    Err(_) => sender.send(Self::WM::UnknownBatteryInfo).expect("todo"),
                };

                sender.send(BatteryWM::ConvervationMode(get_conservation_mode())).unwrap();

                glib::Continue(true)
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

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();

        let label_status = gtk::Label::builder().build();
        holder.pack_end(&label_status, false, false, 0);
        let label_cm = gtk::Label::builder().build();
        holder.pack_end(&label_cm, false, false, 0);
        let value = gtk::Label::builder().build();
        holder.pack_end(&value, false, false, 0);

        let mut receiver = self.dualchannel.get_out_receiver();
        let mut percent = 0;
        let mut cm_str = String::new();
        let mut status_str = String::new();

        MainContext::ref_thread_default().spawn_local(async move {
            if let Ok(msg) = receiver.recv().await {
                match msg {
                    BatteryWM::ConvervationMode(cm) => {
                        let status = match cm {
                            ConvervationMode::Enable => "CMON",
                            ConvervationMode::Disable => "CMDA",
                            ConvervationMode::Unknown => "UKN",
                        };

                        if AsRef::<str>::as_ref(&cm_str) != status {
                            label_cm.set_label(status);
                            cm_str = status.to_string();
                        }
                    },
                    BatteryWM::BatteryInfo(bi) => {
                        let status = bi.get_percent();

                        if status != percent {
                            value.set_label(format!("{}", status).as_str());
                            percent = status;
                        }

                        let status = match bi.status {
                            PowerStatus::NotCharging => "NOT CGR",
                            PowerStatus::Discharging => "DISCGR",
                            PowerStatus::Charging => "CHRGNG",
                            PowerStatus::Unknown => "UNKNWN",
                        };

                        if AsRef::<str>::as_ref(&status_str) != status {
                            label_status.set_label(status);
                            status_str = status.to_string();
                        }      
                    },
                    BatteryWM::UnknownBatteryInfo => todo!(),
                }
            }
        });

        holder.upcast()
    }
}
