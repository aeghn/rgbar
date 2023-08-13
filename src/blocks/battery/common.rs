use super::PowerStatus::{Charging, Discharging, NotCharging, Unknown};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;

use super::{BatteryInfo, PowerStatus};

static POWER_INFO_PATH: &str = "/sys/class/power_supply/BAT0/uevent";

impl BatteryInfo {
    pub fn get_capacity(&self) -> u8 {
        self.capacity
    }

    pub fn get_status(&self) -> &PowerStatus {
        &self.status
    }
}

pub fn read_battery_info() -> BatteryInfo {
    read_event(POWER_INFO_PATH)
}

fn read_event(_path: &str) -> BatteryInfo {
    let mut name: String = "".to_string();
    let mut status: PowerStatus = PowerStatus::Unknown;
    let mut present: u8 = 0;
    let mut technology: String = "".to_string();
    let mut cycle_count: u32 = 0;
    let mut voltage_min_design: u32 = 0;
    let mut voltage_now: u32 = 0;
    let mut power_now: u32 = 0;
    let mut energy_full_design: u32 = 0;
    let mut energy_full: u32 = 0;
    let mut energy_now: u32 = 0;
    let mut capacity: u8 = 0;
    let mut capacity_level: String = "".to_string();
    let mut model_name: String = "".to_string();
    let mut manufacturer: String = "".to_string();
    let mut serial_numer: String = "".to_string();

    // File hosts must exist in current path before this produces output
    if let Ok(lines) = read_lines(POWER_INFO_PATH) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(ip) = line {
                let mut kv = ip.split("=");
                let k = kv.next().unwrap().to_string();
                let v = kv.next().unwrap().to_string();
                if k.eq("POWER_SUPPLY_NAME") {
                    name = v.to_string();
                } else if k.eq("POWER_SUPPLY_STATUS") {
                    if v.eq_ignore_ascii_case("charging") {
                        status = Charging;
                    } else if v.eq_ignore_ascii_case("not charging") {
                        status = NotCharging;
                    } else if v.eq_ignore_ascii_case("discharging") {
                        status = Discharging;
                    } else {
                        status = Unknown;
                    }
                } else if k.eq("POWER_SUPPLY_PRESENT") {
                    present = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_TECHNOLOGY") {
                    technology = v;
                } else if k.eq("POWER_SUPPLY_CYCLE_COUNT") {
                    cycle_count = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_VOLTAGE_MIN_DESIGN") {
                    voltage_min_design = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_VOLTAGE_NOW") {
                    voltage_now = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_POWER_NOW") {
                    power_now = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_ENERGY_FULL_DESIGN") {
                    energy_full_design = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_ENERGY_FULL") {
                    energy_full = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_ENERGY_NOW") {
                    energy_now = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_CAPACITY") {
                    capacity = v.parse().unwrap();
                } else if k.eq("POWER_SUPPLY_CAPACITY_LEVEL") {
                    capacity_level = v;
                } else if k.eq("POWER_SUPPLY_MODEL_NAME") {
                    model_name = v;
                } else if k.eq("POWER_SUPPLY_MANUFACTURER") {
                    manufacturer = v;
                } else if k.eq("POWER_SUPPLY_SERIAL_NUMBER") {
                    serial_numer = v;
                }
            }
        }
    }

    BatteryInfo {
        name,
        status,
        present,
        technology,
        cycle_count,
        voltage_min_design,
        voltage_now,
        power_now,
        energy_full_design,
        energy_full,
        energy_now,
        capacity,
        capacity_level,
        model_name,
        manufacturer,
        serial_numer,
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
