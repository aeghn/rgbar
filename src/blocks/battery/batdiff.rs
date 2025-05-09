use std::time::{SystemTime, UNIX_EPOCH};

use notify_rust::{Notification, Timeout};

use crate::{prelude::StatusName, util::timeutil::second_to_human};

use super::{BatteryInfo, PowerStatus};

#[derive(Debug, Clone, Copy)]
pub(super) struct BatDiff {
    // Power Icon
    pub(super) last_power_status: PowerStatus,

    // Percent number and icon
    pub(super) last_percent: u8,

    // Remain time
    pub(super) energy_diff: usize,
    pub(super) time_diff: usize,
    pub(super) last_record_seconds: usize,
    pub(super) last_record_energy: usize,
    pub(super) last_remain_time_notify_sec: usize,
    pub(super) last_remain_time_label_time: usize,
}

const INTERVAL: usize = 2;

pub fn seconds_now() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize
}

impl BatDiff {
    pub fn check_percent<F>(&mut self, battery_info: &BatteryInfo, callback: F)
    where
        F: Fn(u8, StatusName) -> (),
    {
        let percent = battery_info.get_percent();
        if percent != self.last_percent {
            let mapped = match percent {
                0..=9 => StatusName::BatteryEmpty,
                10..=30 => StatusName::BatteryLow,
                31..=60 => StatusName::BatteryMid,
                61..=99 => StatusName::BatteryHigh,
                _ => StatusName::BatteryFull,
            };

            callback(percent, mapped);

            self.last_percent = percent;
        }
    }

    pub fn check_power_status<F>(&mut self, battery_info: &BatteryInfo, callback: F)
    where
        F: Fn(StatusName) -> (),
    {
        let status = battery_info.status;
        if self.last_power_status != status {
            let mapped = match status {
                PowerStatus::NotCharging => StatusName::BatteryPowerNotCharging,
                PowerStatus::Discharging => StatusName::BatteryPowerDisconnected,
                PowerStatus::Charging => StatusName::BattetyPowerCharging,
                PowerStatus::Full => StatusName::BatteryPowerFull,
                PowerStatus::Unknown => StatusName::BatteryPowerUnknown,
            };
            self.last_power_status = status;

            //
            callback(mapped)
        }
    }
    pub fn check_remain_time<F>(&mut self, battery_info: &BatteryInfo, callback: F)
    where
        F: Fn(Option<usize>) -> (),
    {
        let status = battery_info.status;

        if status != PowerStatus::Discharging {
            callback(None);
            return;
        }

        let seconds_now = seconds_now() as usize;

        let time_cost = seconds_now - self.last_record_seconds;
        self.last_record_seconds = seconds_now;

        // We treat this as a sleep time if it is not updated after 60 seconds.
        if time_cost < INTERVAL + 3 {
            self.time_diff += time_cost;
        }

        let energy_now = battery_info.energy_now as usize;
        let cap_diff = self.last_record_energy.saturating_sub(energy_now);
        self.energy_diff += cap_diff;

        self.last_record_energy = energy_now;

        if self.energy_diff > 0 {
            tracing::info!(
                " {} * {} / {}",
                energy_now,
                self.time_diff,
                self.energy_diff
            );
            let remain_secs = energy_now * self.time_diff / self.energy_diff;
            if battery_info.get_percent() < 30
                && seconds_now - self.last_remain_time_notify_sec > 300
            {
                let _ = Notification::new()
                    .summary("Low Battery")
                    .body(
                        format!(
                            "Connect to adaptar... \nRemain {}% ({})",
                            battery_info.get_percent(),
                            second_to_human(remain_secs)
                        )
                        .as_str(),
                    )
                    .icon("battery-low")
                    .urgency(notify_rust::Urgency::Critical)
                    .timeout(Timeout::Milliseconds(6000)) //milliseconds
                    .show();
                self.last_remain_time_notify_sec = seconds_now;
            }

            if seconds_now.saturating_sub(self.last_remain_time_label_time) > 30 {
                // remain_time.set_label(&format!("({})", second_to_human(remain_secs)));
                callback(Some(remain_secs));
                self.last_remain_time_label_time = seconds_now;
            }
        } else {
            callback(None);
        }
    }
}
