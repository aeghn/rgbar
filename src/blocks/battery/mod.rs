use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::datahodler::channel::DualChannel;

use crate::window::WidgetShareInfo;
use crate::util::gtk_icon_loader::load_fixed_status_surface;
use crate::util::timeutil::second_to_human;

use self::common::get_battery_info;
#[cfg(feature = "ideapad")]
use self::ideapad::{get_conservation_mode, ConvervationMode};

use super::Block;

use crate::prelude::*;
use chin_tools::AResult;


use tracing::warn;

mod common;
#[cfg(feature = "ideapad")]
mod ideapad;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PowerStatus {
    NotCharging = 1,
    Discharging = 2,
    Charging = 3,
    Full = 4,
    Unknown = 5,
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
    #[cfg(feature = "ideapad")]
    ConvervationMode(ConvervationMode),
    BatteryInfo(BatteryInfo),
    UnknownBatteryInfo,
    BatteryPowerDisconnected(usize, usize), // Timestamp, battery
    BatteryPowerConnected,                // Timestamp, battery
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

fn mills() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

impl Block for BatteryBlock {
    type Out = BatteryOut;
    type In = BatteryIn;

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();
        let mut last_info: Option<PowerStatus> = None;

        timeout_add_seconds(
            1,
            clone!(
                
                @strong sender =>
                move || {
                    match get_battery_info() {
                        Ok(info) => {
                            if let PowerStatus::Discharging = info.status {
                                if Some(&PowerStatus::Discharging) != last_info.as_ref() {
                                    let seconds = mills();

                                    sender
                                        .send(Self::Out::BatteryPowerDisconnected(
                                            seconds.try_into().unwrap(),
                                            info.energy_now.clone() as usize,
                                        ))
                                        .expect("send disconnected info");

                                    last_info.replace(PowerStatus::Discharging);
                                }
                            } else if PowerStatus::Charging == info.status
                                || PowerStatus::NotCharging == info.status
                            {
                                if Some(&PowerStatus::Discharging) == last_info.as_ref() {
                                    sender
                                        .send(Self::Out::BatteryPowerConnected)
                                        .expect("unable to send");
                                    last_info.take();
                                }
                            }
                            sender
                                .send(Self::Out::BatteryInfo(info))
                                .expect("send battery info message")
                        }
                        Err(_) => sender.send(Self::Out::UnknownBatteryInfo).expect("todo"),
                    };

                    #[cfg(feature = "ideapad")]
                    sender
                        .send(BatteryOut::ConvervationMode(get_conservation_mode()))
                        .unwrap();

                    ControlFlow::Continue
                }
            ),
        );

        let receiver = self.dualchannel.get_in_receiver();
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

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let battery_status_icon = gtk_icon_loader::load_fixed_status_image(StatusName::BatteryMid);
        battery_status_icon.style_context().add_class("f-20");

        let battery_info = gtk::Label::builder().build();
        battery_info.style_context().add_class("battery-label");
        let remain_time = gtk::Label::builder().build();
        remain_time.style_context().add_class("battery-label");

        #[cfg(feature = "ideapad")]
        let convervation_icon = gtk_icon_loader::load_fixed_status_image(StatusName::BatteryConservationOff);

        let power_status_icon = gtk_icon_loader::load_fixed_status_image(StatusName::BatteryPowerDisconnected);

        holder.pack_start(&battery_status_icon, false, false, 0);
        holder.pack_start(&power_status_icon, false, false, 0);
        #[cfg(feature = "ideapad")]
        holder.pack_start(&convervation_icon, false, false, 0);
        holder.pack_start(&battery_info, false, false, 0);
        holder.pack_start(&remain_time, false, false, 0);

        let mut percent = 0;

        #[cfg(feature = "ideapad")]
        let mut cm_status = ConvervationMode::Unknown;
        let mut power_status = PowerStatus::Unknown;
        let mut last_refresh_label_time = 0;

        let mut disconnect_info: Option<(usize, usize)> = None;

        let mut receiver = self.dualchannel.get_out_receiver();

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        #[cfg(feature = "ideapad")]
                        BatteryOut::ConvervationMode(cm) => {
                            if cm_status != cm {
                                cm_status = cm;
                                let mapped = match cm_status {
                                    ConvervationMode::Enable => StatusName::BatteryConservationOn,
                                    ConvervationMode::Disable => StatusName::BatteryConservationOff,
                                    ConvervationMode::Unknown => {
                                        StatusName::BatteryConservationUnknown
                                    }
                                };
                                convervation_icon.set_from_pixbuf(Some(&load_fixed_from_svg(mapped)))
                            }
                        }
                        BatteryOut::BatteryInfo(bi) => {
                            let status = bi.get_percent();

                            if status != percent {
                                let mapped = match status {
                                    0..=9 => StatusName::BatteryEmpty,
                                    10..=30 => StatusName::BatteryLow,
                                    31..=60 => StatusName::BatteryMid,
                                    61..=99 => StatusName::BatteryHigh,
                                    _ => StatusName::BatteryFull,
                                };

                                battery_status_icon.set_from_surface(load_fixed_status_surface(mapped).as_ref());

                                percent = status;
                            }

                            if let Some((time, value)) = disconnect_info {
                                let seconds = mills();
                                let time_diff = (seconds - time as u64) as f64;

                                let cap_diff = value as u32 - bi.energy_now;
                                if cap_diff > 0 && seconds - last_refresh_label_time > 10 {
                                    let remain_secs = (bi.energy_now as f64
                                        / (cap_diff as f64 / time_diff))
                                        as u32;

                                    remain_time
                                        .set_label(&format!("({})", second_to_human(remain_secs)));

                                    last_refresh_label_time = seconds;
                                }
                            } else {
                                remain_time.set_label("")
                            }

                            battery_info.set_label(&format!("{}%", status));

                            let pstatus = bi.status;

                            if power_status != pstatus {
                                power_status = pstatus;
                                let mapped = match power_status {
                                    PowerStatus::NotCharging => StatusName::BatteryPowerNotCharging,
                                    PowerStatus::Discharging => StatusName::BatteryPowerDisconnected,
                                    PowerStatus::Charging => StatusName::BattetyPowerCharging,
                                    PowerStatus::Full => StatusName::BatteryPowerFull,
                                    PowerStatus::Unknown => StatusName::BatteryPowerUnknown,
                                };

                                power_status_icon.set_from_surface(load_fixed_status_surface(mapped).as_ref());

                            }
                        }
                        BatteryOut::UnknownBatteryInfo => {}
                        BatteryOut::BatteryPowerDisconnected(time, value) => {
                            disconnect_info.replace((time, value));
                        }
                        BatteryOut::BatteryPowerConnected => {
                            disconnect_info.take();
                        }
                    }
                }
            }
        });

        holder.upcast()
    }
}
