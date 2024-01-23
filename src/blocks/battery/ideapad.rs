use std::fs;

static IDEAPAD_ACPI: &str = "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00";
static CONSERVATION_MODE: &str = "conservation_mode";
// static FN_LOCK : &str =  format!("{}/{}", IDEAPAD_ACPI, "fn_lock").as_str();
// static CAMERA_POWER : &str =  format!("{}/{}", IDEAPAD_ACPI, "camera_power").as_str();
// static FAN_MODE : &str =  format!("{}/{}", IDEAPAD_ACPI, "fan_mode").as_str();

#[derive(Clone, PartialEq)]
pub enum ConvervationMode {
    Enable,
    Disable,
    Unknown,
}

pub fn get_conservation_mode() -> ConvervationMode {
    match fs::read_to_string(format!("{}/{}", IDEAPAD_ACPI, CONSERVATION_MODE)) {
        Ok(value) => {
            if value.starts_with("1") {
                ConvervationMode::Enable
            } else {
                ConvervationMode::Disable
            }
        }
        Err(_) => ConvervationMode::Unknown,
    }
}
