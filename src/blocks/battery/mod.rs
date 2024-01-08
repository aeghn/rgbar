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

use crate::datahodler::channel::DualChannel;
use crate::datahodler::channel::MReceiver;
use crate::datahodler::channel::SSender;

use self::common::get_battery_info;
use self::ideapad::get_conservation_mode;
use self::ideapad::ConvervationMode;

use super::Block;
use glib::clone;
use glib::MainContext;
use tracing::warn;

#[derive(Clone)]
pub enum BatteryWM {
    ConvervationMode(ConvervationMode),
    PowerStatus(PowerStatus),
    BatteryInfo(BatteryInfo),
    UnknownBatteryInfo,
}

#[derive(Clone)]
pub enum BatteryBM {}

pub struct BatteryModule {
    dualchannel: DualChannel<BatteryWM, BatteryBM>,
}

impl BatteryModule {
    fn new() -> Self {
        let dualchannel = DualChannel::new(100);

        Self { dualchannel }
    }
}

impl Block for BatteryModule {
    type WM = BatteryWM;
    type BM = BatteryBM;

    fn loop_receive(&mut self) {
        let sender = self.dualchannel.get_out_sender();
        glib::timeout_add_seconds(
            1,
            clone!(@strong sender => move || {
                match get_battery_info() {
                    Ok(info) => sender.send(Self::WM::BatteryInfo(info)).expect("msg"),
                    Err(_) => sender.send(Self::WM::UnknownBatteryInfo).expect("todo"),
                };

                sender.send(BatteryWM::ConvervationMode(get_conservation_mode()));

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
    }

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        todo!()
    }
}
