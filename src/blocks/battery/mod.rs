use crate::datahodler::channel::DualChannel;

use crate::util::gtk_icon_loader::load_fixed_status_surface;
use crate::util::timeutil::second_to_human;
use crate::window::WidgetShareInfo;

use self::common::get_battery_info;
#[cfg(feature = "ideapad")]
use self::ideapad::{get_conservation_mode, ConvervationMode};

use super::Block;

use crate::prelude::*;
use batdiff::seconds_now;
use batdiff::BatDiff;
use chin_tools::AResult;

use gtk::glib::timeout_add_seconds_local_once;

mod batdiff;
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

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();

        macro_rules! begin_fetch {
            ($sender:expr) => {
                match get_battery_info() {
                    Ok(info) => $sender
                        .send(Self::Out::BatteryInfo(info))
                        .expect("send battery info message"),
                    Err(_) => $sender
                        .send(Self::Out::UnknownBatteryInfo)
                        .expect("send battery info message"),
                };

                #[cfg(feature = "ideapad")]
                $sender
                    .send(BatteryOut::ConvervationMode(get_conservation_mode()))
                    .unwrap();
            };
        }

        let sender1 = sender.clone();
        timeout_add_seconds_local_once(
            1,
            clone!(
                @strong sender1 =>
                move || {
                    begin_fetch!(sender1);
                }
            ),
        );

        let sender2 = sender.clone();
        timeout_add_seconds(
            2,
            clone!(
                @strong sender2 =>
                move || {
                    begin_fetch!(sender2);
                    ControlFlow::Continue
                }
            ),
        );

        Ok(())
    }

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let battery_status_icon = gtk_icon_loader::load_fixed_status_image(StatusName::BatteryMid);
        battery_status_icon.style_context().add_class("f-20");

        let percent_label = gtk::Label::builder().build();
        percent_label.style_context().add_class("battery-label");
        let remain_time_label = gtk::Label::builder().build();
        remain_time_label.style_context().add_class("battery-label");

        #[cfg(feature = "ideapad")]
        let convervation_icon =
            gtk_icon_loader::load_fixed_status_image(StatusName::BatteryConservationOff);

        let power_status_icon =
            gtk_icon_loader::load_fixed_status_image(StatusName::BatteryPowerDisconnected);

        holder.pack_start(&battery_status_icon, false, false, 0);
        holder.pack_start(&power_status_icon, false, false, 0);
        #[cfg(feature = "ideapad")]
        holder.pack_start(&convervation_icon, false, false, 0);
        holder.pack_start(&percent_label, false, false, 0);
        holder.pack_start(&remain_time_label, false, false, 0);

        #[cfg(feature = "ideapad")]
        let mut cm_status = ConvervationMode::Unknown;

        let mut receiver = self.dualchannel.get_out_receiver();

        let mut batdiff: Option<BatDiff> = None;

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
                                convervation_icon
                                    .set_from_pixbuf(Some(&load_fixed_from_svg(mapped)))
                            }
                        }
                        BatteryOut::BatteryInfo(bi) => {
                            if let Some(bd) = batdiff.as_mut() {
                                bd.check_percent(&bi, |percent, mapped| {
                                    percent_label.set_label(&format!("{}%", percent));
                                    battery_status_icon.set_from_surface(
                                        load_fixed_status_surface(mapped).as_ref(),
                                    );
                                });

                                bd.check_power_status(&bi, |mapped| {
                                    power_status_icon.set_from_surface(
                                        load_fixed_status_surface(mapped).as_ref(),
                                    );
                                });

                                bd.check_remain_time(&bi, |mapped| {
                                    if let Some(time) = mapped {
                                        remain_time_label
                                            .set_label(&format!("({})", second_to_human(time)));
                                    } else {
                                        remain_time_label.set_label("");
                                    }
                                });
                            } else {
                                batdiff.replace(BatDiff {
                                    last_power_status: PowerStatus::Unknown,
                                    last_percent: 0,
                                    energy_diff: 0,
                                    time_diff: 0,
                                    last_record_seconds: seconds_now(),
                                    last_record_energy: bi.energy_now as usize,
                                    last_remain_time_notify_sec: 0,
                                    last_remain_time_label_time: seconds_now(),
                                });
                            }
                        }
                        BatteryOut::UnknownBatteryInfo => {}
                    }
                }
            }
        });

        holder.upcast()
    }
}
